use super::{
    super::tests::PUB_KEY,
    proof::sub_proof::SubProof,
    read::range_query,
    write::{Apply, WriteContext},
    BPlusTreeNode, BPlusTreeNodeId, BPlusTreeNodeLoader,
};
use crate::chain::bplus_tree::BPlusTreeRoot;
use crate::chain::id_tree::ObjId;
use crate::{
    chain::{range::Range, traits::Num},
    digest::{Digest, Digestible},
};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::num::NonZeroU64;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
struct TestBPlusTree<K: Num> {
    root_id: Option<BPlusTreeNodeId>,
    nodes: HashMap<BPlusTreeNodeId, BPlusTreeNode<K>>,
}

impl<K: Num> BPlusTreeNodeLoader<K> for TestBPlusTree<K> {
    fn load_node(&self, id: BPlusTreeNodeId) -> Result<BPlusTreeNode<K>> {
        match self.nodes.get(&id).cloned() {
            Some(n) => Ok(n),
            None => bail!("Cannot find node in TestBPlusTree"),
        }
    }
}

impl<K: Num> BPlusTreeNodeLoader<K> for &'_ TestBPlusTree<K> {
    fn load_node(&self, id: BPlusTreeNodeId) -> Result<BPlusTreeNode<K>> {
        match self.nodes.get(&id).cloned() {
            Some(n) => Ok(n),
            None => bail!("Cannot find node in TestBPlusTree"),
        }
    }
}

impl<K: Num> TestBPlusTree<K> {
    pub fn new() -> Self {
        Self {
            root_id: None,
            nodes: HashMap::new(),
        }
    }

    fn apply(&mut self, apply: Apply<K>) {
        self.root_id = apply.root.bplus_tree_root_id;
        self.nodes.extend(apply.nodes.into_iter());
    }
}

const FANOUT: usize = 4;

fn get_dataset() -> (Vec<u32>, Vec<NonZeroU64>) {
    // 30 int from 1 to 25 with duplicates
    let keys: Vec<u32> = vec![
        9, 11, 23, 13, 4, 12, 5, 11, 10, 18, 20, 3, 24, 4, 15, 8, 7, 2, 3, 21, 1, 17, 6, 20, 14,
        25, 22, 16, 19, 1,
    ];

    // 30 ids
    unsafe {
        let ids: Vec<NonZeroU64> = vec![
            NonZeroU64::new_unchecked(1),
            NonZeroU64::new_unchecked(2),
            NonZeroU64::new_unchecked(3),
            NonZeroU64::new_unchecked(4),
            NonZeroU64::new_unchecked(5),
            NonZeroU64::new_unchecked(6),
            NonZeroU64::new_unchecked(7),
            NonZeroU64::new_unchecked(8),
            NonZeroU64::new_unchecked(9),
            NonZeroU64::new_unchecked(10),
            NonZeroU64::new_unchecked(11),
            NonZeroU64::new_unchecked(12),
            NonZeroU64::new_unchecked(13),
            NonZeroU64::new_unchecked(14),
            NonZeroU64::new_unchecked(15),
            NonZeroU64::new_unchecked(16),
            NonZeroU64::new_unchecked(17),
            NonZeroU64::new_unchecked(18),
            NonZeroU64::new_unchecked(19),
            NonZeroU64::new_unchecked(20),
            NonZeroU64::new_unchecked(21),
            NonZeroU64::new_unchecked(22),
            NonZeroU64::new_unchecked(23),
            NonZeroU64::new_unchecked(24),
            NonZeroU64::new_unchecked(25),
            NonZeroU64::new_unchecked(26),
            NonZeroU64::new_unchecked(27),
            NonZeroU64::new_unchecked(28),
            NonZeroU64::new_unchecked(29),
            NonZeroU64::new_unchecked(30),
        ];
        (keys, ids)
    }
}

pub fn set_root_id(bplus_tree_root: &mut BPlusTreeRoot, id: Option<BPlusTreeNodeId>) {
    bplus_tree_root.bplus_tree_root_id = id;
}

#[test]
fn test_read() {
    // K is u32
    let mut test_b_tree = TestBPlusTree::<u32>::new();
    let mut test_b_tree_root = BPlusTreeRoot::default();
    set_root_id(&mut test_b_tree_root, test_b_tree.root_id);
    let mut ctx = WriteContext::new(&mut test_b_tree, test_b_tree_root);
    let keys: Vec<u32> = get_dataset().0;
    let ids: Vec<NonZeroU64> = get_dataset().1;

    for i in 0..30 {
        ctx.insert(keys[i], ObjId(ids[i]), FANOUT, &PUB_KEY)
            .unwrap();
    }

    let changes = ctx.changes();
    test_b_tree.apply(changes);

    let root_digest = test_b_tree
        .load_node(test_b_tree.root_id.unwrap())
        .unwrap()
        .to_digest();

    let query_range = Range::new(1, 4);
    let (_v, acc, p) =
        range_query(&test_b_tree, test_b_tree.root_id, query_range, &PUB_KEY).unwrap();

    let res_digest = p.verify(query_range, acc, &PUB_KEY).unwrap();
    assert_eq!(root_digest, res_digest);

    let query_range = Range::new(3, 10);
    let (_v, acc, p) =
        range_query(&test_b_tree, test_b_tree.root_id, query_range, &PUB_KEY).unwrap();
    let res_digest = p.verify(query_range, acc, &PUB_KEY).unwrap();
    assert_eq!(root_digest, res_digest);

    let query_range = Range::new(5, 30);
    let (_v, acc, p) =
        range_query(&test_b_tree, test_b_tree.root_id, query_range, &PUB_KEY).unwrap();
    let res_digest = p.verify(query_range, acc, &PUB_KEY).unwrap();
    assert_eq!(root_digest, res_digest);

    let query_range = Range::new(31, 40);
    let (_v, acc, p) =
        range_query(&test_b_tree, test_b_tree.root_id, query_range, &PUB_KEY).unwrap();
    let res_digest = p.verify(query_range, acc, &PUB_KEY).unwrap();
    assert_eq!(root_digest, res_digest);
}

#[test]
fn test_pointer() {
    let mut query_proof = SubProof::from_hash(Range::new(1, 2), Digest::zero());
    let mut cur_proof = &mut query_proof as *mut _;
    let mut sub_proof_queue: VecDeque<*mut SubProof<i32>> = VecDeque::new();
    sub_proof_queue.push_back(cur_proof);
    cur_proof = sub_proof_queue.pop_front().unwrap();
    println!("Raw pointer address before: {:p}", cur_proof);
    unsafe {
        *cur_proof = SubProof::from_hash(Range::new(2, 3), Digest::zero());
    }
    println!("Raw pointer address after: {:p}", cur_proof);
    assert_eq!(1, 1);
}
