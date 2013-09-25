use extra::sort::Sort;
use std::hashmap;

pub trait CallGraph {
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str]) -> ~[&'l ~str];
    fn fns_directly_affected_by(&self, id: uint) -> ~[uint];
    fn fns_affected_by<'l>(&'l self, id: uint) -> ~[uint] {
        let mut affected_ids = self.fns_directly_affected_by(id);
        let mut ids = ~[];
        while ids != affected_ids {
            ids = affected_ids;
            affected_ids = ids + ids.flat_map(|&i| self.fns_directly_affected_by(i));
            affected_ids.qsort();
            affected_ids.dedup();
        }
        affected_ids
    }
    // fn delete<'l>(&'l mut self, func: &str) -> ~[&'l ~str];
}

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

pub struct CallerToCalleeGraph {
    fns: ~[~str],
    graph: hashmap::HashMap<uint, ~[uint]>,
}

impl CallerToCalleeGraph {
    pub fn new() -> CallerToCalleeGraph {
        CallerToCalleeGraph {
            fns: ~[],
            graph: hashmap::HashMap::new(),
        }
    }
}

impl CallGraph for CallerToCalleeGraph {
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str]) -> ~[&'l ~str] {
        // get func's position in list of maintained fns
        let new_fn_position = match self.fns.position_elem(&func) {
            None => {
                self.fns.push(func);
                self.fns.len() - 1
            },
            Some(p) => p,
        };
        // temporarily remove func from graph
        self.graph.pop(&new_fn_position);
        // update func's dependencies to point to func
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
        // return list of affected fns
        self.fns_affected_by(new_fn_position).map(|&i| &self.fns[i])
    }

    fn fns_directly_affected_by(&self, id: uint) -> ~[uint] {
        self.graph.iter().filter(|&(_, v)| v.contains(&id)).map(|(&k, _)| k).collect()
    }

    // fn delete<'l>(&'l mut self, func: &str) -> ~[&'l ~str] {
    //     match self.fns.position_elem(&func) {
    //         None => ~[], // ignore func if not in maintained fn list
    //         Some(pos) => {
    //             self.graph.pop(pos);
    //             // get list of directly affected fns
    //             self.graph.iter().filter(|(_, v)| v.contains(&pos)).map(|(&k, _)| k).collect()
    //         }
    //     }
    // }
}

pub struct CalleeToCallerGraph {
    fns: ~[~str],
    graph: hashmap::HashMap<uint, ~[uint]>,
}

impl CalleeToCallerGraph {
    pub fn new() -> CalleeToCallerGraph {
        CalleeToCallerGraph {
            fns: ~[],
            graph: hashmap::HashMap::new(),
        }
    }
}

impl CallGraph for CalleeToCallerGraph {
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str]) -> ~[&'l ~str] {
        // get func's position in list of maintained fns
        let new_fn_position = match self.fns.position_elem(&func) {
            None => {
                self.fns.push(func);
                self.fns.len() - 1
            },
            Some(p) => p,
        };
        // get list of affected fns
        let caller_ids = self.fns_affected_by(new_fn_position);
        // update func's dependencies to point to func
        for &d in dependencies.iter() {
            match position(d, self.fns) {
                None => (), // ignore fns which are not in list of maintained fns
                Some(pos) => {
                    do self.graph.insert_or_update_with(pos, ~[new_fn_position])
                        |_, deps| {
                            if !deps.contains(&new_fn_position) {
                                deps.push(new_fn_position);
                            }
                        };
                },
            }
        }
        caller_ids.map(|&i| &self.fns[i])
    }

    fn fns_directly_affected_by(&self, id: uint) -> ~[uint] {
        match self.graph.find(&id) {
            None => ~[],
            Some(p) => p.clone(),
        }
    }
}

pub struct BothWayGraph {
    caller_callee: CallerToCalleeGraph,
    callee_caller: CalleeToCallerGraph,
}

impl BothWayGraph {
    pub fn new() -> BothWayGraph {
        BothWayGraph {
            caller_callee: CallerToCalleeGraph::new(),
            callee_caller: CalleeToCallerGraph::new(),
        }
    }
}

impl CallGraph for BothWayGraph {
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str]) -> ~[&'l ~str] {
        let l1 = self.caller_callee.update(func.clone(), dependencies);
        let l2 = self.callee_caller.update(func, dependencies);
        assert!(l1 == l2);
        l1
    }

    fn fns_directly_affected_by(&self, id: uint) -> ~[uint] {
        self.caller_callee.fns_directly_affected_by(id)
    }
}
