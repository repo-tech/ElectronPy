import ast
import json
import sys


def convert_annotation(node):
    if node is None:
        return None
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        base = convert_annotation(node.value)
        return f"{base}.{node.attr}" if base else node.attr
    if isinstance(node, ast.Subscript):
        value = convert_annotation(node.value)
        slice_value = convert_annotation(node.slice)
        return f"{value}[{slice_value}]" if value and slice_value else value
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if isinstance(node, ast.Call):
        func_name = convert_annotation(node.func)
        return func_name
    return None


class PythonAstExporter(ast.NodeVisitor):
    def visit_Module(self, node):
        return {"type": "module", "body": [self.visit(stmt) for stmt in node.body]}

    def visit_Assign(self, node):
        if len(node.targets) != 1:
            raise ValueError("multiple assignment targets are not supported")
        return {"type": "assign", "target": self.visit(node.targets[0]), "value": self.visit(node.value)}

    def visit_AnnAssign(self, node):
        target = self.visit(node.target)
        value = self.visit(node.value) if node.value is not None else None
        return {"type": "assign", "target": target, "value": value}

    def visit_AugAssign(self, node):
        operators = {ast.Add: "add", ast.Sub: "sub", ast.Mult: "mul", ast.Div: "div", ast.Mod: "mod"}
        op_type = type(node.op)
        if op_type not in operators:
            raise ValueError(f"unsupported augmented assignment operator: {op_type.__name__}")
        return {
            "type": "assign",
            "target": self.visit(node.target),
            "value": {"type": "binary", "left": self.visit(node.target), "operator": operators[op_type], "right": self.visit(node.value)},
        }

    def visit_Expr(self, node):
        return {"type": "expr", "value": self.visit(node.value)}

    def visit_Name(self, node):
        return {"type": "name", "id": node.id}

    def visit_Constant(self, node):
        value = node.value
        if isinstance(value, bool):
            return {"type": "bool", "value": value}
        if isinstance(value, int):
            return {"type": "int", "value": value}
        if isinstance(value, float):
            return {"type": "float", "value": value}
        if isinstance(value, str):
            return {"type": "string", "value": value}
        if value is None:
            return {"type": "none"}
        raise ValueError(f"unsupported constant: {type(value).__name__}")

    def visit_BinOp(self, node):
        operators = {ast.Add: "add", ast.Sub: "sub", ast.Mult: "mul", ast.Div: "div", ast.Mod: "mod", ast.BitAnd: "and", ast.BitOr: "or"}
        op_type = type(node.op)
        if op_type not in operators:
            raise ValueError(f"unsupported operator: {op_type.__name__}")
        return {"type": "binary", "left": self.visit(node.left), "operator": operators[op_type], "right": self.visit(node.right)}

    def visit_Call(self, node):
        return {"type": "call", "function": self.visit(node.func), "args": [self.visit(arg) for arg in node.args], "keywords": [self.visit(keyword) for keyword in node.keywords]}

    def visit_keyword(self, node):
        return {"type": "keyword", "arg": node.arg, "value": self.visit(node.value)}

    def visit_Compare(self, node):
        operators = {ast.Eq: "eq", ast.NotEq: "ne", ast.Lt: "lt", ast.LtE: "le", ast.Gt: "gt", ast.GtE: "ge"}
        ops = []
        for op in node.ops:
            op_type = type(op)
            if op_type not in operators:
                raise ValueError(f"unsupported comparison: {op_type.__name__}")
            ops.append(operators[op_type])
        return {"type": "compare", "left": self.visit(node.left), "operators": ops, "comparators": [self.visit(c) for c in node.comparators]}

    def visit_If(self, node):
        return {"type": "if", "test": self.visit(node.test), "body": [self.visit(x) for x in node.body], "orelse": [self.visit(x) for x in node.orelse]}

    def visit_While(self, node):
        return {"type": "while", "test": self.visit(node.test), "body": [self.visit(x) for x in node.body]}

    def visit_For(self, node):
        return {"type": "for", "target": self.visit(node.target), "iter": self.visit(node.iter), "body": [self.visit(x) for x in node.body]}

    def visit_FunctionDef(self, node):
        arg_annotations = [convert_annotation(arg.annotation) for arg in node.args.args]
        return {
            "type": "funcdef",
            "name": node.name,
            "args": [arg.arg for arg in node.args.args],
            "arg_annotations": arg_annotations,
            "body": [self.visit(x) for x in node.body],
            "returns": convert_annotation(node.returns),
        }

    def visit_Return(self, node):
        return {"type": "return", "value": self.visit(node.value) if node.value is not None else None}

    def visit_List(self, node):
        return {"type": "list", "elements": [self.visit(x) for x in node.elts]}

    def visit_Subscript(self, node):
        return {"type": "subscript", "value": self.visit(node.value), "index": self.visit(node.slice)}

    def generic_visit(self, node):
        raise ValueError(f"unsupported AST node: {type(node).__name__}")


def export_python_ast(source):
    tree = ast.parse(source)
    exporter = PythonAstExporter()
    result = exporter.visit(tree)
    print("Native Python AST Visitor re-enabled via ast.NodeVisitor", file=sys.stderr)
    return json.dumps(result, separators=(",", ":"))


if __name__ == "__main__":
    source = sys.stdin.read()
    print(export_python_ast(source))
