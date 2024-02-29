use cdlist::LinkNode;

#[test]
fn deref_mut() {
    let mut node0 = LinkNode::new(1);
    let node1 = LinkNode::new(2);
    *node0 += *node1;
    assert_eq!(3, *node0);
}

#[test]
fn iter_single() {
    let node0 = LinkNode::new(0);
    assert_eq!(collect(&node0), vec![0]);
}

#[test]
fn iter() {
    let mut nodes = (0..10).map(LinkNode::new).collect::<Vec<_>>();
    connect_all(&mut nodes, 0, 10);
    assert_eq!(collect(&nodes[0]), (0..10).collect::<Vec<_>>());
    assert_eq!(collect_rev(&nodes[9]), (0..10).rev().collect::<Vec<_>>());
}

#[test]
fn iter_mut() {
    let mut nodes = (0..10).map(LinkNode::new).collect::<Vec<_>>();
    connect_all(&mut nodes, 0, 10);
    let mut j = 0;
    nodes[5].for_each_mut(|i| {
        *i += j;
        j += 1;
    });
    j = 0;
    assert_eq!(collect(&nodes[0]), vec![5, 7, 9, 11, 13, 5, 7, 9, 11, 13]);
    nodes[9].for_each_mut_rev(|i| {
        *i += j;
        j += 1;
    });
    assert_eq!(
        collect(&nodes[0]),
        vec![14, 15, 16, 17, 18, 9, 10, 11, 12, 13]
    );
}

#[test]
fn pop_self() {
    let mut node0 = LinkNode::new(0);
    node0.take();
    assert_eq!(collect(&node0), vec![0]);
}

#[test]
fn requeue() {
    let mut n0 = LinkNode::new(0);
    let mut n1 = LinkNode::new(1);
    let mut n2 = LinkNode::new(2);
    n0.add(&mut n1);
    n1.add(&mut n2);
    assert_eq!(collect(&n0), vec![0, 1, 2]);
    n2.add(&mut n1);
    assert_eq!(collect(&n0), vec![0, 2, 1]);
    assert_eq!(collect(&n2), vec![2, 1, 0]);
}

#[test]
fn take() {
    let mut nodes = (0..10).map(LinkNode::new).collect::<Vec<_>>();
    connect_all(&mut nodes, 0, 10);
    assert_eq!(collect(&nodes[0]), (0..10).collect::<Vec<_>>());
    let to_take = [0, 2, 4, 6, 8];
    for i in to_take {
        nodes[i].take();
    }
    for i in to_take {
        assert_eq!(collect(&nodes[i]), vec![i]);
    }
    assert_eq!(collect(&nodes[1]), vec![1, 3, 5, 7, 9]);
}

#[test]
fn add() {
    let mut nodes = (0..10).map(LinkNode::new).collect::<Vec<_>>();
    connect_all(&mut nodes, 0, 5);
    connect_all(&mut nodes, 5, 10);
    assert_eq!(collect(&nodes[0]), (0..5).collect::<Vec<_>>());
    assert_eq!(collect(&nodes[5]), (5..10).collect::<Vec<_>>());
    let (n0, n1) = nodes.split_at_mut(5);
    n0[2].add(&mut n1[2]);
    assert_eq!(collect(&nodes[0]), vec![0, 1, 2, 7, 3, 4]);
    assert_eq!(collect_rev(&nodes[4]), vec![4, 3, 7, 2, 1, 0]);
    assert_eq!(collect(&nodes[5]), vec![5, 6, 8, 9]);
    assert_eq!(collect_rev(&nodes[9]), vec![9, 8, 6, 5]);
}

// helper functions

fn collect<T: Copy>(node: &LinkNode<T>) -> Vec<T> {
    let mut vec = vec![];
    node.for_each(|&i| vec.push(i));
    vec
}

fn collect_rev<T: Copy>(node: &LinkNode<T>) -> Vec<T> {
    let mut vec = vec![];
    node.for_each_rev(|&i| vec.push(i));
    vec
}

fn connect_all<T>(nodes: &mut [LinkNode<T>], start: usize, end: usize) {
    for i in start..(end - 1) {
        let (ni, nj) = nodes[i..].split_at_mut(1);
        ni[0].add(&mut nj[0])
    }
}
