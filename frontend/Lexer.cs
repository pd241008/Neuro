using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;

namespace Neuro.Frontend
{
    public enum TokenType
    {
        Fn, Mut, Let, Type, 
        Int, Float, If, Else, Return, Printf, Scanf,
        Leq, Geq, Id, Num, String, Assign, SemiColon,
        LBrace, RBrace, LParen, RParen, Comma, Multiply, EOF, Unknown,
        Equal, LessThan, GreaterThan, NotEqual, Not, Plus, Minus, Divide,
        LBracket, RBracket, Dot, Colon
    }

    public class Token
    {
        public TokenType Type { get; set; }
        public string Value { get; set; }
        public int Offset { get; set; }

        public Token(TokenType type, string value, int offset = 0) {
            Type = type;
            Value = value;
            Offset = offset;
        }
    }

public class Lexer {
    private readonly string _source;
    private int _pos = 0;

    private readonly Dictionary<string, TokenType> _keywords = new() {
        { "int", TokenType.Int }, { "float", TokenType.Float },
        { "if", TokenType.If }, { "else", TokenType.Else },
        { "return", TokenType.Return }, { "scanf", TokenType.Scanf },
        { "printf", TokenType.Printf }, { "fun", TokenType.Fn }, { "mut", TokenType.Mut },
        { "let", TokenType.Let }, { "type", TokenType.Type }
    };

    public Lexer(string source) => _source = source;

    public List<Token> Tokenize() {
        var tokens = new List<Token>();
        while (_pos < _source.Length) {
            char current = _source[_pos];

            if (char.IsWhiteSpace(current)) { _pos++; continue; }

            // Handle Identifiers and Keywords
            if (char.IsLetter(current)) {
                int startOffset = _pos;
                string word = ReadWhile(char.IsLetterOrDigit);
                tokens.Add(new Token(_keywords.GetValueOrDefault(word, TokenType.Id), word, startOffset));
            }
            // Handle Numbers
            else if (char.IsDigit(current)) {
                int startOffset = _pos;
                tokens.Add(new Token(TokenType.Num, ReadWhile(char.IsDigit), startOffset));
            }
            // Handle Symbols
            else {
                tokens.Add(MatchSymbol());
            }
        }
        tokens.Add(new Token(TokenType.EOF, "", _pos));
        return tokens;
    }

    private string ReadWhile(Predicate<char> condition) {
        int start = _pos;
        while (_pos < _source.Length && condition(_source[_pos])) _pos++;
        return _source[start.._pos];
    }

    private Token MatchSymbol() {
        int startOffset = _pos;
        char c = _source[_pos++];
        return c switch {
  
        '=' => Peek('=') ? Consume('=', TokenType.Equal, startOffset) : new Token(TokenType.Assign, "=", startOffset),
        '<' => Peek('=') ? Consume('=', TokenType.Leq, startOffset)    : new Token(TokenType.LessThan, "<", startOffset),
        '>' => Peek('=') ? Consume('=', TokenType.Geq, startOffset)    : new Token(TokenType.GreaterThan, ">", startOffset),
        '!' => Peek('=') ? Consume('=', TokenType.NotEqual, startOffset): new Token(TokenType.Not, "!", startOffset),
        '+' => new Token(TokenType.Plus, "+", startOffset),
        '-' => new Token(TokenType.Minus, "-", startOffset),
        '*' => new Token(TokenType.Multiply, "*", startOffset),
        '/' => new Token(TokenType.Divide, "/", startOffset),
  
        '(' => new Token(TokenType.LParen, "(", startOffset),
        ')' => new Token(TokenType.RParen, ")", startOffset),
        '{' => new Token(TokenType.LBrace, "{", startOffset),
        '}' => new Token(TokenType.RBrace, "}", startOffset),
        '[' => new Token(TokenType.LBracket, "[", startOffset), // Added for arrays/lists
        ']' => new Token(TokenType.RBracket, "]", startOffset), // Added for arrays/lists
        ';' => new Token(TokenType.SemiColon, ";", startOffset),
        ',' => new Token(TokenType.Comma, ",", startOffset),
        '.' => new Token(TokenType.Dot, ".", startOffset),
        ':' => new Token(TokenType.Colon, ":", startOffset),

          _ => throw new Exception($"Unknown character: {c}")
        };
    }

    private bool Peek(char expected) => _pos < _source.Length && _source[_pos] == expected;
    private Token Consume(char expected, TokenType type, int offset) { _pos++; return new Token(type, "", offset); }
}

}
