//! This module doesn't contain **functions which use nested borrows in their
//! signatures**, and doesn't contain functions with loops.
#![allow(dead_code)]

pub struct Pair<T1, T2> {
    x: T1,
    y: T2,
}

pub enum List<T> {
    Cons(T, Box<List<T>>),
    Nil,
}

/// Sometimes, enumerations with one variant are not treated
/// the same way as the other variants (for example, downcasts
/// are not always introduced).
/// A downcast is the cast of an enum to a specific variant, like
/// in the left value of:
/// `((_0 as Right).0: T2) = move _1;`
pub enum One<T1> {
    One(T1),
}

/// Truely degenerate case
/// Instantiations of this are encoded as constant values by rust.
pub enum EmptyEnum {
    Empty,
}

/// Enumeration (several variants with no parameters)
/// Those are not encoded as constant values.
pub enum Enum {
    Variant1,
    Variant2,
}

/// Degenerate struct
/// Instanciations of this are encoded as constant values by rust.
pub struct EmptyStruct {}

pub enum Sum<T1, T2> {
    Left(T1),
    Right(T2),
}

/// Testing unop simplification
/// In debug mode, rust introduces an assertion before the negation.
fn neg_test(x: i32) -> i32 {
    -x
}

/// Testing binop simplification
/// In debug mode, rust inserts an assertion after the addition
fn add_test(x: u32, y: u32) -> u32 {
    x + y
}

/// Testing binop simplification
/// In debug mode, rust inserts an assertion after the substraction
fn subs_test(x: u32, y: u32) -> u32 {
    x - y
}

/// Testing binop simplification
/// In debug mode, rust inserts an assertion before the division
fn div_test(x: u32, y: u32) -> u32 {
    x / y
}

/// Testing binop simplification
/// When using constants, rustc removes the unnecessary assertions (but
/// only at a specific pass)
fn div_test1(x: u32) -> u32 {
    x / 2
}

/// Testing binop simplification
fn rem_test(x: u32, y: u32) -> u32 {
    x % y
}

fn cast_test(x: u32) -> i32 {
    x as i32
}

#[allow(unused_variables)]
fn test2() {
    let x: u32 = 23;
    let y: u32 = 44;
    let z = x + y;
    let p: Pair<u32, u32> = Pair { x: x, y: z };
    let s: Sum<u32, bool> = Sum::Right(true);
    let o: One<u64> = One::One(3);
    let e0 = EmptyEnum::Empty;
    let e1 = e0;
    let enum0 = Enum::Variant1;
}

fn get_max(x: u32, y: u32) -> u32 {
    if x >= y {
        x
    } else {
        y
    }
}

fn test3() {
    let x = get_max(4, 3);
    let y = get_max(10, 11);
    let z = x + y;
    assert!(z == 15);
}

fn test_neg1() {
    let x: i32 = 3;
    let y = -x;
    assert!(y == -3);
}

/// Testing nested references.
fn refs_test1() {
    let mut x = 0;
    let mut px = &mut x;
    let ppx = &mut px;
    **ppx = 1;
    // The interesting thing happening here is that the borrow of x is inside
    // the borrow of px: ending the borrow of x requires ending the borrow of
    // px first.
    assert!(x == 1);
}

fn refs_test2() {
    let mut x = 0;
    let mut y = 1;
    let mut px = &mut x;
    let py = &mut y;
    let ppx = &mut px;
    *ppx = py;
    **ppx = 2;
    assert!(*px == 2);
    assert!(x == 0);
    assert!(*py == 2);
    assert!(y == 2);
}

/// Box creation
#[allow(unused_variables)]
fn test_list1() {
    let l: List<i32> = List::Cons(0, Box::new(List::Nil));
}

/// Box deref
fn test_box1() {
    use std::ops::Deref;
    use std::ops::DerefMut;
    let mut b: Box<i32> = Box::new(0);
    let x = b.deref_mut();
    *x = 1;
    let x = b.deref();
    assert!(*x == 1);
}

fn copy_int(x: i32) -> i32 {
    x
}

/// Just checking the parameters given to unreachable
/// Rk.: the input parameter prevents using the function as a unit test.
fn test_unreachable(b: bool) {
    if b {
        unreachable!();
    }
}

/// Just checking the parameters given to panic
/// Rk.: the input parameter prevents using the function as a unit test.
fn test_panic(b: bool) {
    if b {
        panic!("Panicked!");
    }
}

// Just testing that shared loans are correctly handled
fn test_copy_int() {
    let x = 0;
    let px = &x;
    let y = copy_int(x);
    assert!(*px == y);
}

fn is_cons<T>(l: &List<T>) -> bool {
    match l {
        List::Cons(_, _) => true,
        List::Nil => false,
    }
}

fn test_is_cons() {
    let l: List<i32> = List::Cons(0, Box::new(List::Nil));

    assert!(is_cons(&l));
}

fn split_list<T>(l: List<T>) -> (T, List<T>) {
    match l {
        List::Cons(hd, tl) => (hd, *tl),
        _ => panic!(),
    }
}

#[allow(unused_variables)]
fn test_split_list() {
    let l: List<i32> = List::Cons(0, Box::new(List::Nil));

    let (hd, tl) = split_list(l);
    assert!(hd == 0);
}

fn choose<'a, T>(b: bool, x: &'a mut T, y: &'a mut T) -> &'a mut T {
    if b {
        return x;
    } else {
        return y;
    }
}

fn choose_test() {
    let mut x = 0;
    let mut y = 0;
    let z = choose(true, &mut x, &mut y);
    *z = *z + 1;
    assert!(*z == 1);
    // drop(z)
    assert!(x == 1);
    assert!(y == 0);
}

/// Test with a char literal - testing serialization
fn test_char() -> char {
    'a'
}

/// Mutually recursive types
enum Tree<T> {
    Leaf(T),
    Node(T, NodeElem<T>, Box<Tree<T>>),
}

enum NodeElem<T> {
    Cons(Box<Tree<T>>, Box<NodeElem<T>>),
    Nil,
}

/*
// TODO: those definitions requires semantic termination (breaks the Coq backend
// because we don't use fuel in this case).

/// Mutually recursive functions
fn even(x: u32) -> bool {
    if x == 0 {
        true
    } else {
        odd(x - 1)
    }
}

fn odd(x: u32) -> bool {
    if x == 0 {
        false
    } else {
        even(x - 1)
    }
}

fn test_even_odd() {
    assert!(even(0));
    assert!(even(4));
    assert!(odd(1));
    assert!(odd(5));
}
*/

pub fn list_length<'a, T>(l: &'a List<T>) -> u32 {
    match l {
        List::Nil => {
            return 0;
        }
        List::Cons(_, l1) => {
            return 1 + list_length(l1);
        }
    }
}

pub fn list_nth_shared<'a, T>(l: &'a List<T>, i: u32) -> &'a T {
    match l {
        List::Nil => {
            panic!()
        }
        List::Cons(x, tl) => {
            if i == 0 {
                return x;
            } else {
                return list_nth_shared(tl, i - 1);
            }
        }
    }
}

pub fn list_nth_mut<'a, T>(l: &'a mut List<T>, i: u32) -> &'a mut T {
    // (i)
    match l {
        List::Nil => {
            panic!()
        }
        List::Cons(x, tl) => {
            // (ii)
            if i == 0 {
                return x; // (iii)
            } else {
                // (iv)
                return list_nth_mut(tl, i - 1);
            }
        }
    }
}

/// In-place list reversal - auxiliary function
fn list_rev_aux<'a, T>(li: List<T>, mut lo: List<T>) -> List<T> {
    match li {
        List::Nil => {
            return lo;
        }
        List::Cons(hd, mut tl) => {
            let next = *tl;
            *tl = lo;
            lo = List::Cons(hd, tl);
            return list_rev_aux(next, lo);
        }
    }
}

/// In-place list reversal
pub fn list_rev<'a, T>(l: &'a mut List<T>) {
    let li = std::mem::replace(l, List::Nil);
    *l = list_rev_aux(li, List::Nil);
}

fn test_list_functions() {
    let mut ls = List::Cons(
        0,
        Box::new(List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))))),
    );
    assert!(list_length(&ls) == 3);
    assert!(*list_nth_shared(&ls, 0) == 0);
    assert!(*list_nth_shared(&ls, 1) == 1);
    assert!(*list_nth_shared(&ls, 2) == 2);
    let x = list_nth_mut(&mut ls, 1);
    *x = 3;
    assert!(*list_nth_shared(&ls, 0) == 0);
    assert!(*list_nth_shared(&ls, 1) == 3); // Updated
    assert!(*list_nth_shared(&ls, 2) == 2);
}

pub fn id_mut_pair1<'a, T1, T2>(x: &'a mut T1, y: &'a mut T2) -> (&'a mut T1, &'a mut T2) {
    (x, y)
}

pub fn id_mut_pair2<'a, T1, T2>(p: (&'a mut T1, &'a mut T2)) -> (&'a mut T1, &'a mut T2) {
    p
}

pub fn id_mut_pair3<'a, 'b, T1, T2>(x: &'a mut T1, y: &'b mut T2) -> (&'a mut T1, &'b mut T2) {
    (x, y)
}

pub fn id_mut_pair4<'a, 'b, T1, T2>(p: (&'a mut T1, &'b mut T2)) -> (&'a mut T1, &'b mut T2) {
    p
}

/// Testing constants (some constants are hard to retrieve from MIR, because
/// they are compiled to very low values).
/// We resort to the following structure to make rustc generate constants...
struct StructWithTuple<T1, T2> {
    p: (T1, T2),
}

fn new_tuple1() -> StructWithTuple<u32, u32> {
    StructWithTuple { p: (1, 2) }
}

fn new_tuple2() -> StructWithTuple<i16, i16> {
    StructWithTuple { p: (1, 2) }
}

fn new_tuple3() -> StructWithTuple<u64, i64> {
    StructWithTuple { p: (1, 2) }
}

/// Similar to [StructWithTuple]
struct StructWithPair<T1, T2> {
    p: Pair<T1, T2>,
}

fn new_pair1() -> StructWithPair<u32, u32> {
    // This actually doesn't make rustc generate a constant...
    // I guess it only happens for tuples.
    StructWithPair {
        p: Pair { x: 1, y: 2 },
    }
}

fn test_constants() {
    assert!(new_tuple1().p.0 == 1);
    assert!(new_tuple2().p.0 == 1);
    assert!(new_tuple3().p.0 == 1);
    assert!(new_pair1().p.x == 1);
}

/// This assignment is trickier than it seems
#[allow(unused_assignments)]
fn test_weird_borrows1() {
    let mut x = 0;
    let mut px = &mut x;
    // Context:
    // x -> [l0]
    // px -> &mut l0 (0:i32)

    px = &mut (*px);
}

fn test_mem_replace(px: &mut u32) {
    let y = std::mem::replace(px, 1);
    assert!(y == 0);
    *px = 2;
}

/// Check that matching on borrowed values works well.
fn test_shared_borrow_bool1(b: bool) -> u32 {
    // Create a shared borrow of b
    let _pb = &b;
    // Match on b
    if b {
        0
    } else {
        1
    }
}

/// Check that matching on borrowed values works well.
/// Testing the concrete execution here.
fn test_shared_borrow_bool2() -> u32 {
    let b = true;
    // Create a shared borrow of b
    let _pb = &b;
    // Match on b
    if b {
        0
    } else {
        1
    }
}

/// Check that matching on borrowed values works well.
/// In case of enumerations, we need to strip the outer loans before evaluating
/// the discriminant.
fn test_shared_borrow_enum1(l: List<u32>) -> u32 {
    // Create a shared borrow of l
    let _pl = &l;
    // Match on l - must ignore the shared loan
    match l {
        List::Nil => 0,
        List::Cons(_, _) => 1,
    }
}

/// Check that matching on borrowed values works well.
/// Testing the concrete execution here.
fn test_shared_borrow_enum2() -> u32 {
    let l: List<u32> = List::Nil;
    // Create a shared borrow of l
    let _pl = &l;
    // Match on l - must ignore the shared loan
    match l {
        List::Nil => 0,
        List::Cons(_, _) => 1,
    }
}
