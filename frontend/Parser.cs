using System;
using System.Collections.Generic;
using Neuro.AST;
using Type = Neuro.AST.Type;

namespace Neuro.Frontend {

public class ParseException : Exception
{
    public string Code { get; }
    public string MessageText { get; }
    public int Offset { get; }
    public int Length { get; }
    public string Label { get; }

    public ParseException(string code, string message, Token? faultToken, string label) : base(message)
    {
        Code = code;
        MessageText = message;
        Offset = faultToken?.Offset ?? 0;
        Length = faultToken?.Value?.Length ?? 1;
        Label = label;
    }
}

public class Parser
{
    private List<Token> _tokens;
    private int _index = 0;

    private Token? CurrentToken => _index < _tokens.Count ? _tokens[_index] : null;

    public Parser(List<Token> tokens)
    {
        _tokens = tokens;
    }

    private SourceLocation GetLocation(Token? token) {
        if (token == null) {
            return new SourceLocation { Line = 1, Column = 1, FilePath = "unknown.nro" };
        }
        return new SourceLocation {
            Line = (uint)token.Line,
            Column = (uint)token.Column,
            FilePath = "unknown.nro"
        };
    }

    public Program ParseProgram()
    {
        Program program = new Program { Name = "MainModule" };

        while (CurrentToken != null && CurrentToken.Type == TokenType.Fn)
        {
            program.Functions.Add(ParseFunction());
        }

        if (CurrentToken != null && CurrentToken.Type != TokenType.EOF)
        {
            throw new ParseException(
                "neuro::syntax::trailing_tokens",
                $"Unexpected token found: '{CurrentToken.Value}'",
                CurrentToken,
                "Trailing characters not allowed"
            );
        }

        return program;
    }

    private Function ParseFunction()
    {
        var fnLoc = GetLocation(CurrentToken);
        Match(TokenType.Fn);
        var nameToken = Match(TokenType.Id);

        var function = new Function {
            Name = nameToken.Value,
            Location = fnLoc
        };

        Match(TokenType.LParen);
        if (CurrentToken?.Type != TokenType.RParen) {
            function.Parameters.Add(ParseParameter());
            while (CurrentToken?.Type == TokenType.Comma) {
                Match(TokenType.Comma);
                function.Parameters.Add(ParseParameter());
            }
        }
        Match(TokenType.RParen);

        Match(TokenType.Minus);
        Match(TokenType.GreaterThan);
        var typeToken = MatchType();
        function.ReturnType = new Type { Kind = ParseType(typeToken.Value) };

        function.Body.AddRange(ParseBlock());
        return function;
    }

    private Parameter ParseParameter() {
        var name = Match(TokenType.Id);
        Match(TokenType.Colon);
        var typeToken = MatchType();
        return new Parameter {
            Name = name.Value,
            Type = new Type { Kind = ParseType(typeToken.Value) }
        };
    }

    private List<Statement> ParseBlock() {
        Match(TokenType.LBrace);
        var stmts = new List<Statement>();
        while (CurrentToken != null && CurrentToken.Type != TokenType.RBrace) {
            stmts.Add(ParseStatement());
        }
        Match(TokenType.RBrace);
        return stmts;
    }

    private Statement ParseStatement() {
        var stmtLoc = GetLocation(CurrentToken);
        var stmt = new Statement { Location = stmtLoc };

        if (CurrentToken?.Type == TokenType.Let) {
            Match(TokenType.Let);
            bool isMut = false;
            if (CurrentToken?.Type == TokenType.Mut) {
                Match(TokenType.Mut);
                isMut = true;
            }
            var id = Match(TokenType.Id);
            Match(TokenType.Colon);
            var typeToken = MatchType();
            Match(TokenType.Assign);
            var expr = ParseExpression();
            Match(TokenType.SemiColon);

            stmt.Declaration = new VariableDeclaration {
                Name = id.Value,
                Type = new Type { Kind = ParseType(typeToken.Value) },
                Initializer = expr,
                IsMutable = isMut
            };
        }
        else if (CurrentToken?.Type == TokenType.If) {
            Match(TokenType.If);
            Match(TokenType.LParen);
            var cond = ParseExpression();
            Match(TokenType.RParen);
            var trueBranch = ParseBlock();
            var falseBranch = new List<Statement>();
            if (CurrentToken?.Type == TokenType.Else) {
                Match(TokenType.Else);
                falseBranch = ParseBlock();
            }
            stmt.IfStmt = new IfStatement {
                Condition = cond,
                TrueBranch = { trueBranch },
                FalseBranch = { falseBranch }
            };
        }
        else if (CurrentToken?.Type == TokenType.While) {
            Match(TokenType.While);
            Match(TokenType.LParen);
            var cond = ParseExpression();
            Match(TokenType.RParen);
            var body = ParseBlock();
            stmt.WhileStmt = new WhileStatement {
                Condition = cond,
                Body = { body }
            };
        }
        else if (CurrentToken?.Type == TokenType.Return) {
            Match(TokenType.Return);
            if (CurrentToken?.Type == TokenType.SemiColon) {
                Match(TokenType.SemiColon);
                stmt.ReturnStmt = new ReturnStatement { Value = null };
            } else {
                var expr = ParseExpression();
                Match(TokenType.SemiColon);
                stmt.ReturnStmt = new ReturnStatement { Value = expr };
            }
        }
        else if (CurrentToken?.Type == TokenType.Id) {
            var id = Match(TokenType.Id);
            if (CurrentToken?.Type == TokenType.Assign) {
                Match(TokenType.Assign);
                var expr = ParseExpression();
                Match(TokenType.SemiColon);
                stmt.Assignment = new Assignment { TargetName = id.Value, Value = expr };
            } else if (CurrentToken?.Type == TokenType.LParen) {
                _index--;
                var expr = ParseExpression();
                Match(TokenType.SemiColon);
                stmt.ExpressionStmt = expr;
            } else {
                throw new ParseException("neuro::syntax::invalid_stmt",
                    $"Unexpected token after identifier '{id.Value}'", CurrentToken, "Here");
            }
        }
        else {
            var expr = ParseExpression();
            Match(TokenType.SemiColon);
            stmt.ExpressionStmt = expr;
        }

        return stmt;
    }

    // --- Expression Parsing (Precedence Climbing) ---

    private Expression ParseExpression() {
        return ParseLogicalOr();
    }

    private Expression ParseLogicalOr() {
        var left = ParseLogicalAnd();
        while (CurrentToken?.Type == TokenType.LogicalOr) {
            var opToken = Match(TokenType.LogicalOr);
            var right = ParseLogicalAnd();
            left = MakeBinary(BinaryOperation.Types.Operator.Or, left, right, opToken);
        }
        return left;
    }

    private Expression ParseLogicalAnd() {
        var left = ParseEquality();
        while (CurrentToken?.Type == TokenType.LogicalAnd) {
            var opToken = Match(TokenType.LogicalAnd);
            var right = ParseEquality();
            left = MakeBinary(BinaryOperation.Types.Operator.And, left, right, opToken);
        }
        return left;
    }

    private Expression ParseEquality() {
        var left = ParseRelational();
        while (CurrentToken?.Type == TokenType.Equal || CurrentToken?.Type == TokenType.NotEqual) {
            var op = CurrentToken.Type == TokenType.Equal
                ? BinaryOperation.Types.Operator.Eq
                : BinaryOperation.Types.Operator.Neq;
            var opToken = Match(CurrentToken.Type);
            var right = ParseRelational();
            left = MakeBinary(op, left, right, opToken);
        }
        return left;
    }

    private Expression ParseRelational() {
        var left = ParseAdditive();
        while (CurrentToken?.Type == TokenType.LessThan || CurrentToken?.Type == TokenType.GreaterThan
            || CurrentToken?.Type == TokenType.Leq || CurrentToken?.Type == TokenType.Geq)
        {
            var op = CurrentToken.Type switch {
                TokenType.LessThan => BinaryOperation.Types.Operator.Lt,
                TokenType.GreaterThan => BinaryOperation.Types.Operator.Gt,
                TokenType.Leq => BinaryOperation.Types.Operator.Lte,
                TokenType.Geq => BinaryOperation.Types.Operator.Gte,
                _ => throw new Exception("unreachable")
            };
            var opToken = Match(CurrentToken.Type);
            var right = ParseAdditive();
            left = MakeBinary(op, left, right, opToken);
        }
        return left;
    }

    private Expression ParseAdditive() {
        var left = ParseMultiplicative();
        while (CurrentToken?.Type == TokenType.Plus || CurrentToken?.Type == TokenType.Minus) {
            var op = CurrentToken.Type == TokenType.Plus
                ? BinaryOperation.Types.Operator.Add
                : BinaryOperation.Types.Operator.Sub;
            var opToken = Match(CurrentToken.Type);
            var right = ParseMultiplicative();
            left = MakeBinary(op, left, right, opToken);
        }
        return left;
    }

    private Expression ParseMultiplicative() {
        var left = ParseUnary();
        while (CurrentToken?.Type == TokenType.Multiply || CurrentToken?.Type == TokenType.Divide) {
            var op = CurrentToken.Type == TokenType.Multiply
                ? BinaryOperation.Types.Operator.Mul
                : BinaryOperation.Types.Operator.Div;
            var opToken = Match(CurrentToken.Type);
            var right = ParseUnary();
            left = MakeBinary(op, left, right, opToken);
        }
        return left;
    }

    private Expression ParseUnary() {
        if (CurrentToken?.Type == TokenType.Minus) {
            var opToken = Match(TokenType.Minus);
            var operand = ParseUnary();
            return new Expression {
                Location = GetLocation(opToken),
                UnaryOp = new UnaryOperation {
                    Op = UnaryOperation.Types.Operator.Neg,
                    Operand = operand
                }
            };
        }
        if (CurrentToken?.Type == TokenType.Not) {
            var opToken = Match(TokenType.Not);
            var operand = ParseUnary();
            return new Expression {
                Location = GetLocation(opToken),
                UnaryOp = new UnaryOperation {
                    Op = UnaryOperation.Types.Operator.Not,
                    Operand = operand
                }
            };
        }
        return ParsePrimary();
    }

    private Expression ParsePrimary() {
        var token = CurrentToken ?? throw new ParseException(
            "neuro::syntax::unexpected_eof", "Unexpected end of file", null, "Expected expression");

        if (token.Type == TokenType.Num) {
            Match(TokenType.Num);
            return new Expression {
                Location = GetLocation(token),
                Literal = new Literal { IntVal = long.Parse(token.Value) }
            };
        }
        if (token.Type == TokenType.FloatNum) {
            Match(TokenType.FloatNum);
            return new Expression {
                Location = GetLocation(token),
                Literal = new Literal { FloatVal = double.Parse(token.Value,
                    System.Globalization.CultureInfo.InvariantCulture) }
            };
        }
        if (token.Type == TokenType.StrLiteral) {
            Match(TokenType.StrLiteral);
            return new Expression {
                Location = GetLocation(token),
                Literal = new Literal { StringVal = token.Value }
            };
        }
        if (token.Type == TokenType.True) {
            Match(TokenType.True);
            return new Expression {
                Location = GetLocation(token),
                Literal = new Literal { BoolVal = true }
            };
        }
        if (token.Type == TokenType.False) {
            Match(TokenType.False);
            return new Expression {
                Location = GetLocation(token),
                Literal = new Literal { BoolVal = false }
            };
        }
        if (token.Type == TokenType.Id) {
            Match(TokenType.Id);
            if (CurrentToken?.Type == TokenType.LParen) {
                return ParseFunctionCall(token);
            }
            return new Expression {
                Location = GetLocation(token),
                Variable = new VariableReference { Name = token.Value }
            };
        }
        if (token.Type == TokenType.LParen) {
            Match(TokenType.LParen);
            var expr = ParseExpression();
            Match(TokenType.RParen);
            return expr;
        }

        throw new ParseException("neuro::syntax::invalid_expr",
            $"Invalid expression starting with '{token.Value}'", token, "Here");
    }

    private Expression ParseFunctionCall(Token idToken) {
        Match(TokenType.LParen);
        var call = new FunctionCall { FunctionName = idToken.Value };
        if (CurrentToken?.Type != TokenType.RParen) {
            call.Arguments.Add(ParseExpression());
            while (CurrentToken?.Type == TokenType.Comma) {
                Match(TokenType.Comma);
                call.Arguments.Add(ParseExpression());
            }
        }
        Match(TokenType.RParen);
        return new Expression {
            Location = GetLocation(idToken),
            Call = call
        };
    }

    private Expression MakeBinary(BinaryOperation.Types.Operator op, Expression left, Expression right, Token opToken) {
        return new Expression {
            Location = GetLocation(opToken),
            BinaryOp = new BinaryOperation {
                Op = op,
                Left = left,
                Right = right
            }
        };
    }

    // --- Type Helpers ---

    private Type.Types.Kind ParseType(string t) {
        return t switch {
            "int" => Type.Types.Kind.Int,
            "float" => Type.Types.Kind.Float,
            "bool" => Type.Types.Kind.Bool,
            "string" => Type.Types.Kind.String,
            "void" => Type.Types.Kind.Void,
            _ => Type.Types.Kind.Custom
        };
    }

    // --- Token Matching ---

    private Token Match(TokenType expected)
    {
        Token? actual = CurrentToken;
        if (actual != null && actual.Type == expected)
        {
            _index++;
            return actual;
        }

        throw new ParseException(
            "neuro::syntax::mismatched_token",
            $"Expected '{expected}', but encountered '{actual?.Type.ToString() ?? "EOF"}'",
            actual ?? (_tokens.Count > 0 ? _tokens[_tokens.Count - 1] : null),
            $"Expected '{expected}' here"
        );
    }

    private Token MatchType()
    {
        Token? actual = CurrentToken;
        if (actual != null && (actual.Type == TokenType.Id
            || actual.Type == TokenType.Int || actual.Type == TokenType.Float
            || actual.Type == TokenType.Bool || actual.Type == TokenType.String
            || actual.Type == TokenType.Void))
        {
            _index++;
            return actual;
        }

        throw new ParseException(
            "neuro::syntax::mismatched_type",
            $"Expected a type, but encountered '{actual?.Type.ToString() ?? "EOF"}'",
            actual ?? (_tokens.Count > 0 ? _tokens[_tokens.Count - 1] : null),
            "Expected a type identifier here"
        );
    }
}
}
