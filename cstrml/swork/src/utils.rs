use sp_std::prelude::*;

pub fn encode_files(fs: &Vec<(Vec<u8>, u64)>) -> Vec<u8> {
    // "["
    let open_square_brackets_bytes: Vec<u8> = [91].to_vec();
    // "\"hash\":\""
    let hash_bytes: Vec<u8> = [123, 34, 104, 97, 115,104, 34, 58, 34].to_vec();
    // "\",\"size\":"
    let size_bytes: Vec<u8> = [34, 44, 34, 115, 105, 122, 101, 34, 58].to_vec();
    // "}"
    let close_curly_brackets_bytes: Vec<u8> = [125].to_vec();
    // ","
    let comma_bytes: Vec<u8> = [44].to_vec();
    // "]"
    let close_square_brackets_bytes: Vec<u8> = [93].to_vec();
    let mut rst: Vec<u8> = open_square_brackets_bytes.clone();
    let len = fs.len();
    for (pos, (hash, size)) in fs.iter().enumerate() {
        rst.extend(hash_bytes.clone());
        rst.extend(encode_file_root(hash.clone()));
        rst.extend(size_bytes.clone());
        rst.extend(encode_u64_to_string_to_bytes(*size));
        rst.extend(close_curly_brackets_bytes.clone());
        if pos != len-1 { rst.extend(comma_bytes.clone()) }
    }

    rst.extend(close_square_brackets_bytes.clone());

    rst
}

// Simulate the process u64.to_string().as_bytes().to_vec()
// eg. 127 -> "127" -> 49 50 55
pub fn encode_u64_to_string_to_bytes(number: u64) -> Vec<u8> {
    let mut value = number;
    let mut encoded_number: Vec<u8> = [].to_vec();
    loop {
        encoded_number.push((value%10) as u8 + 48u8); // "0" is 48u8
        value /= 10;
        if value == 0 {
            break;
        }
    }
    encoded_number.reverse();
    encoded_number
}

// encode file root hash to hex based string
// then represent this string to vec u8
// eg. [91, 92] -> [5b, 5c] -> ["5b", "5c"] -> [53, 98, 53, 99]
fn encode_file_root(fs: Vec<u8>) -> Vec<u8> {
    let mut rst: Vec<u8> = [].to_vec();
    for v in fs.iter() {
        rst.extend(encode_u8_to_hex_string_to_bytes(*v));
    }
    rst
}

// encode one u8 value to hex based string
// then encode this string to vec u8
// eg. 91 -> 5b -> "5b" -> 53 98
fn encode_u8_to_hex_string_to_bytes(number: u8) -> Vec<u8> {
    let upper_value = number / 16 as u8; // 16 is due to hex based
    let lower_value = number % 16 as u8;
    [encode_u8_to_hex_char_to_u8(upper_value), encode_u8_to_hex_char_to_u8(lower_value)].to_vec()
}

// encode 0~16(u8) to hex based char
// then encode this char to corresponding u8
// eg. 5 -> "5" -> 53
// eg. 11 -> "b" -> 98
fn encode_u8_to_hex_char_to_u8(number: u8) -> u8 {
    if number < 10u8 {
        return number + 48u8; // '0' is 48u8
    } else {
        return number - 10u8 + 97u8; // 'a' is 97u8
    }
}