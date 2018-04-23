// Copyright 2018 The rust-dag Authors
// This file is part of the rust-dag library.
//
// The rust-dag library is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// The rust-dag library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with the rust-dag library. If not, see <http://www.gnu.org/licenses/>.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc,RwLock};

use blockdag::{Block,Node,MaxMin,append_maps,tips_anticone,tips_anticone_blue};

/// Function providing blue block calculation.
///
///     input 'block': a new added block to be calculated. before call this function, tips must have been updated for this new block.
///
pub fn calc_blue(block_name: &str, node: &mut Node, k: i32){

    println!("calc_blue(): block {}. func enter.", block_name);

    let dag = &node.dag;
    let block = dag.get(block_name);
    if block.is_none() {
        println!("calc_blue(): error! block {} not exist in dag.", block_name);
        return;

    }

    let block_r = block.unwrap().read().unwrap();

    if block_r.name=="Genesis" {
        drop(block_r);
        let mut block_w = dag.get(block_name).unwrap().write().unwrap();
        block_w.is_blue = true;
        return;
    }

    // step 2
    let mut tip_max_name = String::from(block_name.clone());
    let mut max_past_blue: u64 = block_r.size_of_past_blue;
    let tips = &node.tips;
    if tips.len() == 0 {
        println!("calc_blue(): error! tips must not be empty.");
        return;
    }

    for (key, value) in tips {
//        if key.eq(&tip_max_name) {
//            continue;
//        }
//        println!("calc_blue(): block {}. come to tip {}", block_name, key);

        let tip = &value.read().unwrap();
        if &tip.name == &tip_max_name {
            continue;
        }
        println!("calc_blue(): block {}. come to tip {}", block_name, key);

        if tip.size_of_past_blue > max_past_blue {
            max_past_blue = tip.size_of_past_blue;
            tip_max_name = tip.name.clone();
        }
    }

    println!("calc_blue(): block {}.tip_max_name={},max_past_blue={}", block_name, tip_max_name, max_past_blue);

    // step 3
    if &tip_max_name == block_name {

        println!("calc_blue(): block {}. new block is the max past blue", block_name);

        // step 4
        drop(block_r);

        {
            let (blues, blue_anticone) = tips_anticone_blue(block_name, tips, k);
            if blues >= 0 && blues <= k {
                // step 4.1
                let mut block_w = dag.get(block_name).unwrap().write().unwrap();
                block_w.is_blue = true;
                block_w.size_of_anticone_blue = blues;
                drop(block_w);
                println!("calc_blue(): block {}. add {} to the blue. size_of_anticone_blue={}", block_name, block_name, blues);

                // step 4.2
                check_blue(&blue_anticone, k);
            } else {
                println!("calc_blue(): block {}. error! should be blue, but anticone blues={}", block_name, blues);
            }
            drop(blue_anticone);
        }

        // step 5
        let block_r = dag.get(block_name).unwrap().read().unwrap();
        for (name, value) in &block_r.prev {

            let pred = value.read().unwrap();
            if pred.is_blue {
                continue;
            }
            drop(pred);

            // step 6
            {
                let (blues, blue_anticone) = tips_anticone_blue(name, &block_r.prev, k);
                if blues >= 0 && blues <= k {
                    // step 7
                    let mut pred = value.write().unwrap();
                    pred.is_blue = true;
                    pred.size_of_anticone_blue = blues;
                    println!("calc_blue(): block {}. add {} to the blue. size_of_anticone_blue={}", block_name, pred.name, blues);
                    drop(pred);

                    // step 8
                    check_blue(&blue_anticone, k);
                }
                drop(blue_anticone);
            }
        }
    }else{

        println!("calc_blue(): block {}. new block is not the max past blue", block_name);

        // step 11
        let (blues,blue_anticone) = tips_anticone_blue(block_name, tips, k);
        if blues>=0 && blues<=k {

            drop(block_r);
            let mut block_w = dag.get(block_name).unwrap().write().unwrap();

            // step 12
            //let pred = &Arc::clone(value).write().unwrap();
            block_w.is_blue = true;
            block_w.size_of_anticone_blue = blues;
            println!("calc_blue(): block {}. add {} to the blue. size_of_anticone_blue={}", block_name, block_w.name, blues);
            drop(block_w);

            // step 13
            check_blue(&blue_anticone, k);
        }
    }


}

fn check_blue(blue_anticone: &HashMap<String, Arc<RwLock<Block>>>, k: i32) {

    for (key, value) in blue_anticone {

        let mut block = value.write().unwrap();
        if block.size_of_anticone_blue >= k {
            block.is_blue = false;
            block.size_of_anticone_blue = -1;
            println!("check_blue(): remove {} from blue", block.name);

            dec_successors_anticone_blue(&block);
        }else if block.is_blue{
            block.size_of_anticone_blue += 1;
            println!("check_blue(): {} size_of_anticone_blue increase to {}", block.name, block.size_of_anticone_blue);
        }
    }
}

/// Function update all successors (recursively) of this block, if it's blue, size_of_past_blue minus 1.
///
/// todo: this iteration could be terrible in performance!
///
fn dec_successors_anticone_blue(block: &Block){

    for (key, value) in &block.next {

        let mut next = value.write().unwrap();
        if next.is_blue {
            next.size_of_past_blue -= 1;
        }
        dec_successors_anticone_blue(&next);
    }
}