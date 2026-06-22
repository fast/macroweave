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

use macrotable::repeat;
use macrotable::splice;

#[derive(Debug, PartialEq, Eq)]
enum MetricValue {
    Unsigned(u128),
}

trait IntoMetricValue {
    fn into_metric_value(self) -> MetricValue;
}

repeat!(#T in [u8, u16, u32, u64, usize] {
    impl IntoMetricValue for #T {
        fn into_metric_value(self) -> MetricValue {
            MetricValue::Unsigned(self as u128)
        }
    }
});

struct WorkerStats {
    queued: usize,
    running: usize,
    failed: usize,
}

impl WorkerStats {
    fn counters(&self) -> [(&'static str, usize); 3] {
        splice!(#field in [queued, running, failed] {
            [ #( (stringify!(#field), self.#field) ),* ]
        })
    }
}

fn main() {
    let stats = WorkerStats {
        queued: 4,
        running: 2,
        failed: 1,
    };

    assert_eq!(42u16.into_metric_value(), MetricValue::Unsigned(42));
    assert_eq!(
        stats.counters(),
        [("queued", 4), ("running", 2), ("failed", 1)]
    );

    for (name, value) in stats.counters() {
        println!("{name}: {value}");
    }
}
