//! This Rust module provides a data structure for creating
//! and managing a non-thread-safe doubly-linked list.
//! The core of the module is the `LinkNode<T>` struct,
//! which represents a node in the linked list.
//! Each `LinkNode<T>` contains user-defined data and links to
//! the previous and next nodes in the list.
//!
//! The list is intrusive, meaning that the linked list pointers
//! are stored within the data structure itself, rather than in
//! separate nodes that contain the data as payload.
use std::{
    marker::PhantomData,
    mem::{offset_of, MaybeUninit},
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::{self, NonNull},
};

/// Represents a node in a doubly-linked list.
/// Contains user data of type `T` and links to the previous
/// and next nodes in the list.
///
/// It is designed to be self-referential and is pinned on the heap
/// to ensure its memory safety.
///
/// The data structure is not thread safe.
/// It is not even safe to move to another thread.
/// (!Send and !Sync for whatever type of T).
///
/// Not sync.
/// ```compile_fail,E0277
/// use cdlist::LinkNode;
/// use std::sync::atomic::AtomicUsize;
///
/// fn impl_sync<T: Sync>(val: T) {}
/// /// should not compile
/// impl_sync(LinkNode::new(AtomicUsize::new(0)));
/// ```
///
/// Not send.
/// ```compile_fail,E0277
/// use cdlist::LinkNode;
/// use std::sync::atomic::AtomicUsize;
///
/// fn impl_send<T: Send>(val: T) {}
/// /// should not compile
/// impl_send(LinkNode::new(AtomicUsize::new(0)));
/// ```
pub struct LinkNode<T>(Pin<Box<Inner<T>>>);

/// A private struct used by `LinkNode` to hold
/// the user data and the links to the next and previous
/// nodes in the list. This struct is not exposed outside
/// the module.
///
/// Pinned on heap for linking.
///
/// T can be !Unpin.
struct Inner<T> {
    data: T,
    list: ListHead<T>,
}

/// A private struct that represents the head of the linked list.
/// It contains "prev" and "next" links that may be uninitialized.
struct ListHead<T> {
    prev: MaybeUninit<NonNull<ListHead<T>>>,
    next: MaybeUninit<NonNull<ListHead<T>>>,
    dtype: PhantomData<T>,
}

impl<T> LinkNode<T> {
    /// Creates a new `LinkNode` with the provided user data.
    /// Initializes the node as a standalone element,
    /// effectively creating a new list.
    #[inline]
    pub fn new(data: T) -> Self {
        let mut node = Self(Box::pin(Inner {
            data,
            list: ListHead {
                prev: MaybeUninit::uninit(),
                next: MaybeUninit::uninit(),
                dtype: PhantomData,
            },
        }));
        unsafe {
            node.list_mut().init_head();
        }
        node
    }

    /// Removes `other` from its current position in its list
    /// and inserts it after `self` in the current list.
    #[inline]
    pub fn add(&mut self, other: &mut LinkNode<T>) {
        let self_list = self.list_mut();
        let other_list = other.list_mut();
        unsafe {
            other_list.delist();
            self_list.add(other_list);
        }
    }

    /// Adds `self` to the list of `other`.
    /// It's a convenience method that effectively calls `other.add(self)`.
    #[inline]
    pub fn add_to(&mut self, other: &mut LinkNode<T>) {
        other.add(self)
    }

    /// Removes `self` from its current list,
    /// turning it into a standalone element.
    #[inline]
    pub fn take(&mut self) {
        let list = self.list_mut();
        unsafe {
            list.delist();
            list.init_head();
        }
    }

    /// Iterates over each element in the list starting from `self`
    /// and applies function `f` to an immutable reference
    /// to each element's data.
    pub fn for_each<F>(&self, f: F)
    where
        F: FnMut(&T),
    {
        self.list().for_each(f)
    }

    /// Iterates over each element in the list starting from `self`
    /// and applies function `f` to a mutable reference
    /// to each element's data.
    pub fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T),
    {
        self.list_mut().for_each_mut(f)
    }

    /// Iterates over each element in the list starting from `self`
    /// in reverse order and applies function `f` to an immutable reference
    /// to each element's data.
    pub fn for_each_rev<F>(&self, f: F)
    where
        F: FnMut(&T),
    {
        self.list().for_each_rev(f)
    }

    /// Iterates over each element in the list starting from `self`
    /// in reverse order and applies function `f` to a mutable reference
    /// to each element's data.
    pub fn for_each_mut_rev<F>(&mut self, f: F)
    where
        F: FnMut(&mut T),
    {
        self.list_mut().for_each_rev_mut(f)
    }

    #[inline(always)]
    fn list(&self) -> &ListHead<T> {
        &self.0.list
    }

    #[inline(always)]
    fn list_mut(&mut self) -> &mut ListHead<T> {
        unsafe { &mut self.0.as_mut().get_unchecked_mut().list }
    }
}

impl<T> DerefMut for LinkNode<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.0.as_mut().get_unchecked_mut().data }
    }
}

impl<T> Deref for LinkNode<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<T> Drop for LinkNode<T> {
    fn drop(&mut self) {
        unsafe { self.list_mut().delist() };
    }
}

impl<T> ListHead<T> {
    #[inline(always)]
    unsafe fn ptr(&mut self) -> NonNull<ListHead<T>> {
        NonNull::from(self)
    }

    /// Initializes the list head, setting the previous and
    /// next pointers to point to itself, effectively creating an empty list.
    #[inline(always)]
    unsafe fn init_head(&mut self) {
        let self_ptr = self.ptr();
        self.prev.write(self_ptr);
        self.next.write(self_ptr);
    }

    /// Removes the current node from its list by updating the
    /// previous and next nodes to point to each other.
    /// This method leaves the current node in an inconsistent state
    /// and should be followed by reinsertion into a list using `add` or
    /// resetting the pointers using `init_head`.
    #[inline(always)]
    unsafe fn delist(&mut self) {
        let mut prev = self.prev.assume_init();
        let mut next = self.next.assume_init();
        prev.as_mut().next.write(next);
        next.as_mut().prev.write(prev);
    }

    /// Inserts `other` between `self` and the node currently following `self`.
    /// Assumes `other` is not part of any list.
    #[inline(always)]
    unsafe fn add(&mut self, other: &mut ListHead<T>) {
        let self_ptr = self.ptr();
        let other_ptr = other.ptr();
        let next_ptr = self.next.assume_init();
        let next = self.next.assume_init_mut().as_mut();

        other.prev.write(self_ptr);
        other.next.write(next_ptr);
        next.prev.write(other_ptr);
        self.next.write(other_ptr);
    }

    #[inline(always)]
    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        let self_ptr = ptr::from_ref(self);
        let mut this = self;
        loop {
            f(this.get());
            let next = unsafe { this.next.assume_init_ref() };
            if ptr::addr_eq(next.as_ptr(), self_ptr) {
                break;
            }
            this = unsafe { next.as_ref() };
        }
    }

    #[inline(always)]
    fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        let self_ptr = ptr::from_ref(self);
        let mut this = self;
        loop {
            f(this.get_mut());
            let next = unsafe { this.next.assume_init_mut() };
            if ptr::addr_eq(next.as_ptr(), self_ptr) {
                break;
            }
            this = unsafe { next.as_mut() };
        }
    }

    #[inline(always)]
    fn for_each_rev<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        let self_ptr = ptr::from_ref(self);
        let mut this = self;
        loop {
            f(this.get());
            let prev = unsafe { this.prev.assume_init_ref() };
            if ptr::addr_eq(prev.as_ptr(), self_ptr) {
                break;
            }
            this = unsafe { prev.as_ref() };
        }
    }

    #[inline(always)]
    fn for_each_rev_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T),
    {
        let self_ptr = ptr::from_ref(self);
        let mut this = self;
        loop {
            f(this.get_mut());
            let prev = unsafe { this.prev.assume_init_mut() };
            if ptr::addr_eq(prev.as_ptr(), self_ptr) {
                break;
            }
            this = unsafe { prev.as_mut() };
        }
    }

    /// Returns an immutable reference to the data contained in the
    /// `Inner<T>` struct associated with `self`.
    #[inline(always)]
    fn get(&self) -> &T {
        unsafe { &self.inner().data }
    }

    /// Returns a mutable reference to the data contained in the
    /// `Inner<T>` struct associated with `self`.
    #[inline(always)]
    fn get_mut(&mut self) -> &mut T {
        unsafe { &mut self.inner_mut().data }
    }

    #[inline(always)]
    unsafe fn inner(&self) -> &Inner<T> {
        &*(ptr::from_ref(self)
            .byte_offset(Self::offset())
            .cast::<Inner<T>>())
    }

    #[inline(always)]
    unsafe fn inner_mut(&mut self) -> &mut Inner<T> {
        &mut *(ptr::from_mut(self)
            .byte_offset(Self::offset())
            .cast::<Inner<T>>())
    }

    /// The compiler will compile this into an inlined constant
    /// even without inline const feature.
    #[inline(always)]
    const fn offset() -> isize {
        -(offset_of!(Inner<T>, list) as isize)
    }
}
