(** The primitive values. *)

(** We use big integers to store the integer values (this way we don't have
    to think about the bounds, nor architecture issues - Rust allows to
    manipulate 128-bit integers for instance).
 *)
type big_int = Z.t

let big_int_of_yojson (json : Yojson.Safe.t) : (big_int, string) result =
  match json with
  | `Int i -> Ok (Z.of_int i)
  | `Intlit is -> Ok (Z.of_string is)
  | _ -> Error "not an integer or an integer literal"

let big_int_to_yojson (i : big_int) = `Intlit (Z.to_string i)

let pp_big_int (fmt : Format.formatter) (bi : big_int) : unit =
  Format.pp_print_string fmt (Z.to_string bi)

let compare_big_int (bi0 : big_int) (bi1 : big_int) : int = Z.compare bi0 bi1
let show_big_int (bi : big_int) : string = Z.to_string bi

type integer_type =
  | Isize
  | I8
  | I16
  | I32
  | I64
  | I128
  | Usize
  | U8
  | U16
  | U32
  | U64
  | U128
[@@deriving show, ord]

(** Ancestor the literal_type iter visitor *)
class ['self] iter_literal_type_base =
  object (_self : 'self)
    inherit [_] VisitorsRuntime.iter
    method visit_integer_type : 'env -> integer_type -> unit = fun _ _ -> ()
  end

(** Ancestor the literal_type map visitor *)
class ['self] map_literal_type_base =
  object (_self : 'self)
    inherit [_] VisitorsRuntime.map

    method visit_integer_type : 'env -> integer_type -> integer_type =
      fun _ x -> x
  end

(** Ancestor the literal_type reduce visitor *)
class virtual ['self] reduce_literal_type_base =
  object (self : 'self)
    inherit [_] VisitorsRuntime.reduce

    method visit_integer_type : 'env -> integer_type -> 'a =
      fun _ _ -> self#zero
  end

(** Ancestor the literal_type mapreduce visitor *)
class virtual ['self] mapreduce_literal_type_base =
  object (self : 'self)
    inherit [_] VisitorsRuntime.mapreduce

    method visit_integer_type : 'env -> integer_type -> integer_type * 'a =
      fun _ x -> (x, self#zero)
  end

(* TODO: make literal_type consistent with literal: "integer" or "scalar" *)
type literal_type = Integer of integer_type | Bool | Char
[@@deriving
  show,
    ord,
    visitors
      {
        name = "iter_literal_type";
        variety = "iter";
        ancestors = [ "iter_literal_type_base" ];
        nude = true;
        concrete = true;
      },
    visitors
      {
        name = "map_literal_type";
        variety = "map";
        ancestors = [ "map_literal_type_base" ];
        nude = true;
        concrete = true;
      },
    visitors
      {
        name = "reduce_literal_type";
        variety = "reduce";
        ancestors = [ "reduce_literal_type_base" ];
        nude = true;
      },
    visitors
      {
        name = "mapreduce_literal_type";
        variety = "mapreduce";
        ancestors = [ "mapreduce_literal_type_base" ];
        nude = true;
      }]

(** A scalar value

    Note that we use unbounded integers everywhere.
    We then harcode the boundaries for the different types.
 *)
type scalar_value = { value : big_int; int_ty : integer_type }
[@@deriving show, ord]

(** Ancestor the literal iter visitor *)
class ['self] iter_literal_base =
  object (_self : 'self)
    inherit [_] VisitorsRuntime.iter
    method visit_scalar_value : 'env -> scalar_value -> unit = fun _ _ -> ()
  end

(** Ancestor the literal map visitor *)
class ['self] map_literal_base =
  object (_self : 'self)
    inherit [_] VisitorsRuntime.map

    method visit_scalar_value : 'env -> scalar_value -> scalar_value =
      fun _ x -> x
  end

(** A literal value.

    Can be used by operands (in which case it represents a constant) or by
    the interpreter to represent a concrete, literal value.
 *)
type literal = Scalar of scalar_value | Bool of bool | Char of char
[@@deriving
  show,
    ord,
    visitors
      {
        name = "iter_literal";
        variety = "iter";
        ancestors = [ "iter_literal_base" ];
        nude = true;
        concrete = true;
      },
    visitors
      {
        name = "map_literal";
        variety = "map";
        ancestors = [ "map_literal_base" ];
        nude = true;
        concrete = true;
      }]
