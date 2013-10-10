use extra::sort::Sort;
use std::hashmap;

pub static DEP_NOT_IN_GRAPH: &'static str = "Dependency not in graph";
pub static FN_NOT_IN_GRAPH: &'static str = "Function not in graph";

pub trait CallGraph {
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str])
        -> Result<~[&'l ~str], &str>;
    fn delete(&mut self, func: &str) -> Result<~[~str], &str>;
    fn fns<'l>(&'l self) -> &'l ~[~str];
    fn fns_directly_affected_by(&self, id: uint) -> ~[uint];

    fn add(&mut self, func: ~str, dependencies: &[&str]) -> Result<(), &str> {
        assert!(!self.contains(&[func.as_slice()]));
        match self.update(func, dependencies) {
            Ok(affected) => {
                assert!(affected.len() == 0);
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    fn fns_affected_by(&self, id: uint) -> ~[uint] {
        let mut affected_ids = self.fns_directly_affected_by(id);
        let mut ids = ~[];
        while ids != affected_ids {
            ids = affected_ids;
            affected_ids = ids + do ids.flat_map |&i| {
                self.fns_directly_affected_by(i)
            };
            affected_ids.qsort();
            affected_ids.dedup();
        }
        affected_ids
    }

    fn contains(&self, fns: &[&str]) -> bool {
        for &f in fns.iter() {
            match position(f, *self.fns()) {
                None => return false,
                Some(*) => (),
            }
        }
        return true;
    }
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
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str])
        -> Result<~[&'l ~str], &str> {
        // check if all dependencies are in graph
        if !self.contains(dependencies) {
            return Err(DEP_NOT_IN_GRAPH);
        }
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
                None => unreachable!(),
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
        Ok(self.fns_affected_by(new_fn_position).map(|&i| &self.fns[i]))
    }

    fn delete(&mut self, func: &str) -> Result<~[~str], &str> {
        if !self.contains(&[func]) {
            return Err(FN_NOT_IN_GRAPH);
        }
        let fn_position = position(func, self.fns).unwrap();
        let affected_ids = self.fns_affected_by(fn_position);
        // remove func and its deps from graph
        self.graph.pop(&fn_position);
        for id in affected_ids.iter() {
            self.graph.pop(id);
        }
        let affected = affected_ids.map(|&i| self.fns[i].clone());
        do self.fns.retain |s| {
            if func == *s {
                false
            } else {
                affected.position_elem(s).is_none()
            }
        }
        Ok(affected)
    }

    fn fns_directly_affected_by(&self, id: uint) -> ~[uint] {
        self.graph.iter().filter(|&(_, v)| v.contains(&id)).map(|(&k, _)| k).collect()
    }

    fn fns<'l>(&'l self) -> &'l ~[~str] {
        &self.fns
    }
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
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str])
        -> Result<~[&'l ~str], &str> {
        // check if all dependencies are in graph
        if !self.contains(dependencies) {
            return Err(DEP_NOT_IN_GRAPH);
        }
        // get func's position in list of maintained fns
        let new_fn_position = match self.fns.position_elem(&func) {
            None => {
                self.fns.push(func);
                self.fns.len() - 1
            },
            Some(p) => p,
        };
        // get list of affected fns
        let mut caller_ids = self.fns_affected_by(new_fn_position);
        // update func's dependencies to point to func
        for &d in dependencies.iter() {
            match position(d, self.fns) {
                None => unreachable!(),
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
        caller_ids.push_all(self.fns_affected_by(new_fn_position));
        caller_ids.qsort();
        caller_ids.dedup();
        Ok(caller_ids.map(|&i| &self.fns[i]))
    }

    fn delete(&mut self, func: &str) -> Result<~[~str], &str> {
        if !self.contains(&[func]) {
            return Err(FN_NOT_IN_GRAPH);
        }
        let fn_position = position(func, self.fns).unwrap();
        let affected_ids = self.fns_affected_by(fn_position);
        // remove func and its deps from graph
        self.graph.pop(&fn_position);
        for id in affected_ids.iter() {
            self.graph.pop(id);
        }
        let affected = affected_ids.map(|&i| self.fns[i].clone());
        do self.fns.retain |s| {
            if func == *s {
                false
            } else {
                affected.position_elem(s).is_none()
            }
        }
        Ok(affected)
    }

    fn fns_directly_affected_by(&self, id: uint) -> ~[uint] {
        match self.graph.find(&id) {
            None => ~[],
            Some(p) => p.clone(),
        }
    }

    fn fns<'l>(&'l self) -> &'l ~[~str] {
        &self.fns
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
    fn update<'l>(&'l mut self, func: ~str, dependencies: &[&str])
        -> Result<~[&'l ~str], &str> {
        let l1 = self.caller_callee.update(func.clone(), dependencies);
        let l2 = self.callee_caller.update(func, dependencies);
        assert!(l1 == l2);
        l1
    }

    fn delete(&mut self, func: &str) -> Result<~[~str], &str> {
        let l1 = self.caller_callee.delete(func);
        let l2 = self.callee_caller.delete(func);
        assert!(l1 == l2);
        l1
    }

    fn fns_directly_affected_by(&self, id: uint) -> ~[uint] {
        self.caller_callee.fns_directly_affected_by(id)
    }

    fn fns<'l>(&'l self) -> &'l ~[~str] {
        self.caller_callee.fns()
    }
}
