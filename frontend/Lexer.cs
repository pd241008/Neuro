using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace Neuro.Frontend
{
    public enum TokenType
    {
        Int, Float, If, Else, Return, Printf, Scanf,
        Leq, Id, Num, String, Assign, SemiColon,
        LBrace, RBrace, LParen, RParen, Comma, Multiply, EOF, Unknown
    }

    public class Token
    {
        public TokenType Type { get; set; }
        public string Value { get; set; }

        public Token(TokenType type, string value) {
            Type = type;
            Value = value;
        }
    }

public class Lexer {
    private readonly string _source;
    private int _pos = 0;

    // Map keywords to their token types
    private readonly Dictionary<string, TokenType> _keywords = new() {
        { "int", TokenType.Int }, { "float", TokenType.Float },
        { "if", TokenType.If }, { "else", TokenType.Else },
        { "return", TokenType.Return }, { "scanf", TokenType.Scanf },
        { "printf", TokenType.Printf }
    };

    public Lexer(string source) => _source = source;

    public List<Token> Tokenize() {
        var tokens = new List<Token>();
        while (_pos < _source.Length) {
            char current = _source[_pos];

            if (char.IsWhiteSpace(current)) { _pos++; continue; }

            // Handle Identifiers and Keywords
            if (char.IsLetter(current)) {
                string word = ReadWhile(char.IsLetterOrDigit);
                tokens.Add(new Token(_keywords.GetValueOrDefault(word, TokenType.Id), word));
            }
            // Handle Numbers
            else if (char.IsDigit(current)) {
                tokens.Add(new Token(TokenType.Num, ReadWhile(char.IsDigit)));
            }
            // Handle Symbols
            else {
                tokens.Add(MatchSymbol());
            }
        }
        tokens.Add(new Token(TokenType.EOF, ""));
        return tokens;
    }

    private string ReadWhile(Predicate<char> condition) {
        int start = _pos;
        while (_pos < _source.Length && condition(_source[_pos])) _pos++;
        return _source[start.._pos];
    }

    private Token MatchSymbol() {
        char c = _source[_pos++];
        return c switch {
            '=' => Peek('=') ? Consume('=', TokenType.Leq) : new Token(TokenType.Assign, "="),
            '<' => Peek('=') ? Consume('=', TokenType.Leq) : throw new Exception("Unexpected <"),
            '(' => new Token(TokenType.LParen, "("),
            ')' => new Token(TokenType.RParen, ")"),
            '{' => new Token(TokenType.LBrace, "{"),
            '}' => new Token(TokenType.RBrace, "}"),
            '*' => new Token(TokenType.Multiply, "*"),
            ';' => new Token(TokenType.SemiColon, ";"),
            ',' => new Token(TokenType.Comma, ","),
            _ => throw new Exception($"Unknown character: {c}")
        };
    }

    private bool Peek(char expected) => _pos < _source.Length && _source[_pos] == expected;
    private Token Consume(char expected, TokenType type) { _pos++; return new Token(type, ""); }
}

}
