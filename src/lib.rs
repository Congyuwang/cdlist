use std::{
    marker::PhantomData,
    mem::{offset_of, MaybeUninit},
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::{self, NonNull},
};

/// Owned cell for data.
pub struct LinkNode<T>(Pin<Box<Inner<T>>>);

/// Pinned on heap for linking.
///
/// T can be !Unpin.
struct Inner<T> {
    data: T,
    list: ListHead<T>,
}

/// Intrusive link.
struct ListHead<T> {
    prev: MaybeUninit<NonNull<ListHead<T>>>,
    next: MaybeUninit<NonNull<ListHead<T>>>,
    dtype: PhantomData<T>,
}

impl<T> LinkNode<T> {
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

    /// Pop `other` from its list and add it to `self` list.
    #[inline]
    pub fn add(&mut self, other: &mut LinkNode<T>) {
        let other_list = other.list_mut();
        let self_list = self.list_mut();
        unsafe {
            other_list.delist();
            self_list.add(other_list);
        }
    }

    /// Add `self` to `other` list.
    #[inline]
    pub fn add_to(&mut self, other: &mut LinkNode<T>) {
        other.add(self)
    }

    /// Pop `self` from its current list.
    #[inline]
    pub fn take(&mut self) {
        let list = self.list_mut();
        unsafe {
            list.delist();
            list.init_head();
        }
    }

    pub fn for_each<F>(&self, f: F)
    where
        F: FnMut(&T),
    {
        self.list().for_each(f)
    }

    pub fn for_each_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T),
    {
        self.list_mut().for_each_mut(f)
    }

    pub fn for_each_rev<F>(&self, f: F)
    where
        F: FnMut(&T),
    {
        self.list().for_each_rev(f)
    }

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

    /// Called when node is created.
    #[inline(always)]
    unsafe fn init_head(&mut self) {
        let self_ptr = self.ptr();
        self.prev.write(self_ptr);
        self.next.write(self_ptr);
    }

    /// Unlink the current node from its previous list.
    ///
    /// This is an incomplete operation:
    ///
    /// After this operation, the `prev` and `next` pointers
    /// still points to previous linked list.
    /// Thus, `delist()` should only be used in `drop()`
    /// or in `add()` to update to `prev` and `next` pointers.
    #[inline(always)]
    unsafe fn delist(&mut self) {
        let mut prev = self.prev.assume_init();
        let mut next = self.next.assume_init();
        prev.as_mut().next.write(next);
        next.as_mut().prev.write(prev);
    }

    /// Add `other` between `self` and `self.next`.
    ///
    /// This is an incomplete operation:
    ///
    /// We assume that other has been `delist()` from
    /// its previous chain, so that its previous chain
    /// is still complete.
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
        let self_ptr = self as *const Self;
        let mut this = self;
        loop {
            f(this.get());
            let next = unsafe { this.next.assume_init_ref() };
            if ptr::eq(next.as_ptr(), self_ptr) {
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
        let self_ptr = self as *const Self;
        let mut this = &mut *self;
        loop {
            f(this.get_mut());
            let next = unsafe { this.next.assume_init_mut() };
            if ptr::eq(next.as_ptr(), self_ptr) {
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
        let self_ptr = self as *const Self;
        let mut this = self;
        loop {
            f(this.get());
            let prev = unsafe { this.prev.assume_init_ref() };
            if ptr::eq(prev.as_ptr(), self_ptr) {
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
        let self_ptr = self as *const Self;
        let mut this = &mut *self;
        loop {
            f(this.get_mut());
            let prev = unsafe { this.prev.assume_init_mut() };
            if ptr::eq(prev.as_ptr(), self_ptr) {
                break;
            }
            this = unsafe { prev.as_mut() };
        }
    }

    #[inline(always)]
    fn get(&self) -> &T {
        &unsafe { self.inner() }.data
    }

    #[inline(always)]
    fn get_mut(&mut self) -> &mut T {
        &mut unsafe { self.inner_mut() }.data
    }

    #[inline(always)]
    unsafe fn inner(&self) -> &Inner<T> {
        &*((self as *const Self).byte_offset(Self::offset()) as *const Inner<T>)
    }

    #[inline(always)]
    unsafe fn inner_mut(&mut self) -> &mut Inner<T> {
        &mut *((self as *mut Self).byte_offset(Self::offset()) as *mut Inner<T>)
    }

    #[inline(always)]
    const fn offset() -> isize {
        -(offset_of!(Inner<T>, list) as isize)
    }
}
