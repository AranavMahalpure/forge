(class_declaration
  name: (identifier) @name.definition.class) @definition.class

(class_declaration
  (base_list (_) @name.reference.class)) @reference.class

(interface_declaration
  name: (identifier) @name.definition.interface) @definition.interface

(interface_declaration
  (base_list (_) @name.reference.interface)) @reference.interface

(method_declaration
  name: (identifier) @name.definition.method) @definition.method

(object_creation_expression
  type: (identifier) @name.reference.class) @reference.class

(type_parameter_constraints_clause
  (identifier) @name.reference.class) @reference.class

(type_parameter_constraint
  (type type: (identifier) @name.reference.class)) @reference.class

(variable_declaration
  type: (identifier) @name.reference.class) @reference.class

(invocation_expression
  function: (member_access_expression
    name: (identifier) @name.reference.call)) @reference.call

(namespace_declaration
  name: (identifier) @name.definition.module) @definition.module