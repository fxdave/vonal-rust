/// sort the array and stop when it finds the first N biggest element
pub fn sort<T: PartialOrd>(arr: &mut [T], limit: usize) {
    for i in 0..limit {
        let max_idx = get_max_idx(&arr, i);
        arr.swap(i, max_idx);
    }
}

fn get_max_idx<T: PartialOrd>(arr: &[T], from: usize) -> usize {
    let mut max_idx = from;
    for i in (from + 1)..arr.len() {
        if arr[max_idx] < arr[i] {
            max_idx = i
        }
    }
    return max_idx;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sort() {
        let mut arr = vec![5, 7, 2, 5, 6, 2, 22, 5, 1, 4, 7, 2];
        sort(&mut arr, 3);
        assert_eq!(arr[0], 22);
        assert_eq!(arr[1], 7);
        assert_eq!(arr[2], 7);
    }
}
