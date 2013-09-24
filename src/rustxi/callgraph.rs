use std::hashmap;

fn position(s: &str, l: &[~str]) -> Option<uint> {
    if l.len() > 0 {
        let mut i = 0u;
        for e in l.iter() {
            if s == *e {
                return Some(i);
            }
            i += 1;
        }
    }
    return None;
}

pub struct CallGraph {
    fns: ~[~str],
    graph: hashmap::HashMap<uint, ~[uint]>,
}

impl CallGraph {
    pub fn new() -> CallGraph {
        CallGraph {
            fns: ~[],
            graph: hashmap::HashMap::new(),
        }
    }

    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str]) -> ~[&'l ~str] {
        let new_fn_position = match self.fns.position_elem(&func) {
            None => {
                self.fns.push(func);
                self.fns.len() - 1
            },
            Some(p) => p,
        };
        self.graph.pop(&new_fn_position);
        for &d in dependencies.iter() {
            match position(d, self.fns) {
                None => (), // ignore fns which are not in list of maintained fns
                Some(pos) => {
                    do self.graph.insert_or_update_with(new_fn_position, ~[pos])
                        |_, deps| {
                            if !deps.contains(&pos) {
                                deps.push(pos);
                            }
                        };
                },
            }
        }
        let mut ret = ~[];
        for (&k, v) in self.graph.iter() {
            if v.contains(&new_fn_position) {
                ret.push(&self.fns[k]);
            }
        }
        ret
    }
}
