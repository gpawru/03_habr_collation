/// записать результат как последовательность u16
#[inline(always)]
pub fn output_weights(from: &Vec<u32>) -> Vec<u16>
{
    let mut primary = vec![];
    let mut secondary = vec![];
    let mut tetriary = vec![];

    // веса L1, L2, L3 / маркер переменного веса как u32
    // 1111 1111  1111 1111    2222 2222  2333 33v_

    for &weights in from.iter() {
        let l1 = weights as u16;
        let l2 = (weights >> 16) as u16 & 0x1FF;
        let l3 = (weights >> 25) as u16 & 0x1F;

        if l1 != 0 {
            primary.push(l1);
        };

        if l2 != 0 {
            secondary.push(l2);
        }

        if l3 != 0 {
            tetriary.push(l3);
        }
    }

    primary.push(0);
    primary.extend(secondary);
    primary.push(0);
    primary.extend(tetriary);

    primary
}
