use crate::slice::trie::TrieNode;

#[derive(Debug, Clone, Copy)]
pub struct CollationElement
{
    pub ccc: u8,
    pub code: u32,
    pub value: CollationElementValue,
}

#[derive(Debug, Clone, Copy)]
pub enum CollationElementValue
{
    SingleWeights(u32),
    Decomposition(u16),
    Trie(u16),
    TrieWeights(u16, u8),
}

impl From<TrieNode> for CollationElement
{
    /// элемент сопоставления из trie
    #[inline(always)]
    fn from(node: TrieNode) -> Self
    {
        Self {
            ccc: node.ccc(),
            code: node.code(),
            value: CollationElementValue::TrieWeights(node.weights_offset(), node.weights_len()),
        }
    }
}

impl CollationElement
{
    /// стартер?
    #[inline(always)]
    pub fn is_starter(&self) -> bool
    {
        self.ccc == 0
    }
}
