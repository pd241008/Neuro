using System;
using System.Collections.Generic;
using System.Text.Json;
using Neuro.AST;
using Type = Neuro.AST.Type; // Disambiguate System.Type

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

    public string ToMietteJson()
    {
        var errorPayload = new {
            code = Code,
            message = MessageText,
            labels = new[] {
                new { label = Label, span = new { offset = Offset, length = Length } }
            }
        };
        return JsonSerializer.Serialize(errorPayload);
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

    private SourceLocation GetLocation() {
        return new SourceLocation {
            Line = 1,
            Column = 1,
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
        var fnLoc = GetLocation();
        Match(TokenType.Fn);
        var nameToken = Match(TokenType.Id);
        
        var function = new Function {
            Name = nameToken.Value,
            Location = fnLoc
        };

        Match(TokenType.LParen);
        // Param list simplified for now
        Match(TokenType.RParen);
        
        Match(TokenType.Minus);
        Match(TokenType.GreaterThan);
        var typeToken = Match(TokenType.Id);
        function.ReturnType = new Type { Kind = ParseType(typeToken.Value) };

        Match(TokenType.LBrace);
        
        while (CurrentToken != null && CurrentToken.Type != TokenType.RBrace) {
            function.Body.Add(ParseStatement());
        }
        
        Match(TokenType.RBrace);
        return function;
    }

    private Statement ParseStatement() {
        var stmt = new Statement { Location = GetLocation() };
        
        if (CurrentToken?.Type == TokenType.Let) {
            Match(TokenType.Let);
            if (CurrentToken?.Type == TokenType.Mut) Match(TokenType.Mut);
            var id = Match(TokenType.Id);
            Match(TokenType.Colon);
            var typeToken = Match(TokenType.Id);
            Match(TokenType.Assign);
            var expr = ParseExpression();
            Match(TokenType.SemiColon);

            stmt.Declaration = new VariableDeclaration {
                Name = id.Value,
                Type = new Type { Kind = ParseType(typeToken.Value) },
                Initializer = expr
            };
        }
        else if (CurrentToken?.Type == TokenType.Return) {
            Match(TokenType.Return);
            var expr = ParseExpression();
            Match(TokenType.SemiColon);
            stmt.ReturnStmt = new ReturnStatement { Value = expr };
        }
        else if (CurrentToken?.Type == TokenType.Id) {
            var id = Match(TokenType.Id);
            Match(TokenType.Assign);
            var expr = ParseExpression();
            Match(TokenType.SemiColon);
            stmt.Assignment = new Assignment { TargetName = id.Value, Value = expr };
        }
        else {
            throw new ParseException("neuro::syntax::invalid_stmt", "Invalid statement", CurrentToken, "Here");
        }
        
        return stmt;
    }

    private Expression ParseExpression() {
        var expr = new Expression { Location = GetLocation() };
        
        if (CurrentToken?.Type == TokenType.Num) {
            var num = Match(TokenType.Num);
            expr.Literal = new Literal { IntVal = long.Parse(num.Value) };
        }
        else if (CurrentToken?.Type == TokenType.Id) {
            var id = Match(TokenType.Id);
            expr.Variable = new VariableReference { Name = id.Value };
        }
        else {
            throw new ParseException("neuro::syntax::invalid_expr", "Invalid expression", CurrentToken, "Here");
        }
        return expr;
    }

    private Type.Types.Kind ParseType(string t) {
        return t switch {
            "int" => Type.Types.Kind.Int,
            "float" => Type.Types.Kind.Float,
            "bool" => Type.Types.Kind.Bool,
            _ => Type.Types.Kind.Custom
        };
    }

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
}
}
