use std::marker::PhantomData;

/// итератор по узлам дерева
pub struct TrieIter<'a>
{
    ptr: *const u32,
    start: *const u32,
    is_first: bool,
    _marker: PhantomData<&'a u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct TrieNode
{
    value: u32,
    pos: u16,
}

impl TrieNode
{
    #[inline(always)]
    pub fn from_slice(source: &[u32], pos: u16) -> Self
    {
        Self {
            value: source[pos as usize],
            pos,
        }
    }

    #[inline(always)]
    pub fn from_value(value: u32, pos: u16) -> Self
    {
        Self { value, pos }
    }

    #[inline(always)]
    pub fn code(&self) -> u32
    {
        (self.value >> 8) & 0x3FFFF
    }

    #[inline(always)]
    pub fn ccc(&self) -> u8
    {
        (self.value >> 2) as u8 & 0x3F
    }

    #[inline(always)]
    pub fn is_starter(&self) -> bool
    {
        self.ccc() == 0
    }

    #[inline(always)]
    pub fn pos(&self) -> u16
    {
        self.pos
    }

    #[inline(always)]
    pub fn weights_offset(&self) -> u16
    {
        self.pos + 1
    }

    #[inline(always)]
    pub fn weights_len(&self) -> u8
    {
        (self.value >> 26) as u8
    }

    #[inline(always)]
    pub fn has_children(&self) -> bool
    {
        (self.value & 1) != 0
    }

    #[inline(always)]
    pub fn next_offset(&self) -> u16
    {
        self.pos + self.weights_len() as u16 + 1
    }
}

impl<'a> TrieIter<'a>
{
    #[inline(always)]
    pub fn new(source: &'a [u32], offset: usize) -> Self
    {
        Self {
            ptr: unsafe { source.as_ptr().add(offset) },
            start: source.as_ptr(),
            is_first: true,
            _marker: PhantomData,
        }
    }

    /// следующий элемент на уровне
    #[inline(always)]
    pub fn next(&mut self) -> Option<TrieNode>
    {
        if self.is_first {
            self.is_first = false;

            return Some(self.current_node());
        }

        // текущий элемент - последний?
        if (unsafe { *self.ptr } & 2) != 0 {
            return None;
        }

        // промотаем до следующего элемента
        self.skip_to_next();

        Some(self.current_node())
    }

    /// промотка до следующего элемента без проверок
    #[inline(always)]
    fn skip_to_next(&mut self)
    {
        let mut value = unsafe { *self.ptr };
        let mut level = 0;

        loop {
            // новый элемент располагается на следующем уровне
            if (value & 1) != 0 {
                level += 1;
            }

            // пропускаем запись и веса
            let weights_len = (value >> 26) as usize;
            self.ptr = unsafe { self.ptr.add(1 + weights_len) };

            value = unsafe { *self.ptr };

            // уровень = 0, т.е. полученный элемент - искомый
            if level == 0 {
                return;
            }

            // прочитанный элемент - последний на своем уровне
            if (value & 2) != 0 {
                level -= 1;
            }
        }
    }

    /// текущий узел
    #[inline(always)]
    pub fn current_node(&self) -> TrieNode
    {
        unsafe { TrieNode::from_value(*self.ptr, self.ptr.offset_from(self.start) as u16) }
    }
}
