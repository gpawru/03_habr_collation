use core::marker::PhantomData;

use super::TrieNode;

/// итератор по узлам бора весов
pub struct TrieIter<'a>
{
    ptr: *const u32,
    start: *const u32,
    is_first: bool,
    _marker: PhantomData<&'a u32>,
}

impl<'a> Iterator for TrieIter<'a>
{
    type Item = TrieNode;

    /// следующий потомок узла
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item>
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
}

impl<'a> TrieIter<'a>
{
    /// получить следующий потомок, в случае None - выполнить код
    #[inline(always)]
    pub fn next_or_else<F>(&mut self, on_none: F) -> Option<TrieNode>
    where
        F: FnOnce(),
    {
        match self.next() {
            None => {
                on_none();
                None
            }
            value => value,
        }
    }

    /// итератор по узлам бора весов
    #[inline(always)]
    pub fn new(source: &'a [u32], offset: u16) -> Self
    {
        Self {
            ptr: unsafe { source.as_ptr().add(offset as usize) },
            start: source.as_ptr(),
            is_first: true,
            _marker: PhantomData,
        }
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

    /// текущий узел в более удобном виде
    #[inline(always)]
    pub fn current_node(&self) -> TrieNode
    {
        unsafe { TrieNode::new(*self.ptr, self.ptr.offset_from(self.start) as u16) }
    }
}
