// Copyright 2026 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use macroweave::repeat;
use macroweave::splice;

trait ReadLe {
    fn read_le(input: &[u8]) -> Self;
}

repeat!((Ty, Width) in [
    (u16, 2),
    (u32, 4),
    (u64, 8),
] {
    impl ReadLe for Ty {
        fn read_le(input: &[u8]) -> Self {
            Ty::from_le_bytes(input[..Width].try_into().unwrap())
        }
    }
});

fn keyword_code(text: &str) -> Option<u8> {
    splice!((Pat, Code) in [
        ("async", 1u8),
        ("await", 2u8),
    ] {
        match text {
            #(Pat => Some(Code)),*,
            _ => None,
        }
    })
}

fn main() {
    assert_eq!(u16::read_le(&[0x34, 0x12]), 0x1234);
    assert_eq!(u32::read_le(&[1, 0, 0, 0]), 1);

    assert_eq!(keyword_code("async"), Some(1));
    assert_eq!(keyword_code("await"), Some(2));
    assert_eq!(keyword_code("fn"), None);

    println!("u16: {:#x}", u16::read_le(&[0x34, 0x12]));
    println!("async: {:?}", keyword_code("async"));
}
