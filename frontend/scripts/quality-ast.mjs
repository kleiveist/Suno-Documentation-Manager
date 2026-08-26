import fs from "node:fs";

import ts from "typescript";

const files = process.argv.slice(2);
const payload = { classes: [], functions: [], imports: [] };
let parseFailed = false;

function lineNumber(source, position) {
  return source.getLineAndCharacterOfPosition(position).line + 1;
}

function stringArgument(node) {
  if (!node || (!ts.isStringLiteral(node) && !ts.isNoSubstitutionTemplateLiteral(node))) {
    return null;
  }
  return node.text;
}

function recordImport(source, file, specifierNode) {
  const specifier = stringArgument(specifierNode);
  if (specifier === null) return;
  payload.imports.push({
    file,
    line: lineNumber(source, specifierNode.getStart(source)),
    specifier
  });
}

function localFunctionName(source, node) {
  if (node.name) return node.name.getText(source);
  if (ts.isConstructorDeclaration(node)) return "constructor";
  if (ts.isVariableDeclaration(node.parent)) return node.parent.name.getText(source);
  if (ts.isPropertyAssignment(node.parent) || ts.isPropertyDeclaration(node.parent)) {
    return node.parent.name.getText(source);
  }
  return "<anonymous function>";
}

function assignedExpressionName(source, node, fallback) {
  if (node.name) return node.name.getText(source);
  if (ts.isVariableDeclaration(node.parent)) return node.parent.name.getText(source);
  if (ts.isPropertyAssignment(node.parent) || ts.isPropertyDeclaration(node.parent)) {
    return node.parent.name.getText(source);
  }
  return fallback;
}

function containerName(source, node) {
  if (ts.isModuleDeclaration(node)) return node.name.getText(source);
  if (ts.isClassDeclaration(node) || ts.isClassExpression(node)) {
    return assignedExpressionName(source, node, "<anonymous class>");
  }
  if (ts.isObjectLiteralExpression(node)) {
    return assignedExpressionName(source, node, "<anonymous object>");
  }
  if (
    ts.isFunctionDeclaration(node) ||
    ts.isFunctionExpression(node) ||
    ts.isArrowFunction(node) ||
    ts.isMethodDeclaration(node) ||
    ts.isGetAccessorDeclaration(node) ||
    ts.isSetAccessorDeclaration(node) ||
    ts.isConstructorDeclaration(node)
  ) {
    return localFunctionName(source, node);
  }
  return null;
}

function enclosingSymbolParts(source, node) {
  const parts = [];
  let parent = node.parent;
  while (parent) {
    const name = containerName(source, parent);
    if (name !== null) parts.push(name);
    parent = parent.parent;
  }
  return parts.reverse();
}

function functionName(source, node) {
  return [...enclosingSymbolParts(source, node), localFunctionName(source, node)].join(".");
}

function className(source, node) {
  const localName = assignedExpressionName(source, node, "<anonymous class>");
  return [...enclosingSymbolParts(source, node), localName].join(".");
}

function recordFunction(source, file, node) {
  const functionLike =
    ts.isFunctionDeclaration(node) ||
    ts.isFunctionExpression(node) ||
    ts.isArrowFunction(node) ||
    ts.isMethodDeclaration(node) ||
    ts.isGetAccessorDeclaration(node) ||
    ts.isSetAccessorDeclaration(node) ||
    ts.isConstructorDeclaration(node);
  if (!functionLike) return;
  payload.functions.push({
    file,
    line: lineNumber(source, node.getStart(source)),
    endLine: lineNumber(source, node.getEnd()),
    symbol: functionName(source, node)
  });
}

function visit(source, file, node) {
  if (ts.isClassDeclaration(node) || ts.isClassExpression(node)) {
    payload.classes.push({
      file,
      line: lineNumber(source, node.getStart(source)),
      endLine: lineNumber(source, node.getEnd()),
      symbol: className(source, node)
    });
  }
  recordFunction(source, file, node);
  if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) {
    recordImport(source, file, node.moduleSpecifier);
  }
  if (ts.isCallExpression(node)) {
    const dynamicImport = node.expression.kind === ts.SyntaxKind.ImportKeyword;
    const commonJsRequire = ts.isIdentifier(node.expression) && node.expression.text === "require";
    if (dynamicImport || commonJsRequire) recordImport(source, file, node.arguments[0]);
  }
  ts.forEachChild(node, (child) => visit(source, file, child));
}

for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
  if (source.parseDiagnostics.length > 0) {
    parseFailed = true;
    for (const diagnostic of source.parseDiagnostics) {
      const position = diagnostic.start ?? 0;
      const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, " ");
      process.stderr.write(`${file}:${lineNumber(source, position)}: ${message}\n`);
    }
    continue;
  }
  visit(source, file, source);
}

process.stdout.write(JSON.stringify(payload));
if (parseFailed) process.exitCode = 1;
