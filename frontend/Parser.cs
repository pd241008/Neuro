using System;
using System.Collections.Generic;

namespace Neuro.Frontend {
public class Parser 
{
    private List<Token> tokens;
    private int index = 0;
    private Dictionary<TokenType, Action> statementRules;

    public Parser(List<Token> tokens) 
    {
        this.tokens = tokens;
        statementRules = new Dictionary<TokenType, Action> 
        {
            { TokenType.Scanf, ParseScanf },
            { TokenType.Printf, ParsePrintf },
            { TokenType.Id, ParseAssignment },
            { TokenType.If, ParseIf },
            { TokenType.Return, ParseReturn },
            { TokenType.LBrace, ParseBlock }
        };
    }

    public void ParseProgram() 
    {
        Match(TokenType.Int);
        Match(TokenType.Id);
        Match(TokenType.LParen);
        Match(TokenType.RParen);
        Match(TokenType.LBrace);
        ParseDecls();
        ParseStmts();
        Match(TokenType.RBrace);
    }

    private void ParseDecls() 
    {
        if (index < tokens.Count && tokens[index].Type == TokenType.Float) 
        {
            Match(TokenType.Float);
            ParseIdList();
            Match(TokenType.SemiColon);
        }
    }




    private void ParseIdList() 
    {
        Match(TokenType.Id);
        while (index < tokens.Count && tokens[index].Type == TokenType.Comma) 
        {
            Match(TokenType.Comma);
            Match(TokenType.Id);
        }
    }

    private void ParseStmts() 
    {
        while (index < tokens.Count && statementRules.ContainsKey(tokens[index].Type)) 
        {
            statementRules[tokens[index].Type]();
        }
    }

    private void ParseExpr() 
    {
        if (tokens[index].Type == TokenType.Id) 
        {
            Match(TokenType.Id);
        } 
        else 
        {
            Match(TokenType.Num);
        }

        if (index < tokens.Count && tokens[index].Type == TokenType.Multiply) 
        {
            Match(TokenType.Multiply);
            ParseExpr();
        }
    }

    private void ParseIf() 
    {
        Match(TokenType.If);
        Match(TokenType.LParen);
        ParseExpr();
        if (tokens[index].Type == TokenType.Leq) Match(TokenType.Leq);
        ParseExpr();
        Match(TokenType.RParen);
        ParseStmt();
        if (index < tokens.Count && tokens[index].Type == TokenType.Else) 
        {
            Match(TokenType.Else);
            ParseStmt();
        }
    }

    private void ParseStmt() 
    {
        if (statementRules.ContainsKey(tokens[index].Type)) 
        {
            statementRules[tokens[index].Type]();
        }
    }

    private void ParseAssignment() 
    {
        Match(TokenType.Id);
        Match(TokenType.Assign);
        ParseExpr();
        Match(TokenType.SemiColon);
    }

    private void ParseScanf() { Match(TokenType.Scanf); Match(TokenType.SemiColon); }
    private void ParsePrintf() { Match(TokenType.Printf); Match(TokenType.SemiColon); }
    private void ParseReturn() { Match(TokenType.Return); ParseExpr(); Match(TokenType.SemiColon); }
    private void ParseBlock() { Match(TokenType.LBrace); ParseStmts(); Match(TokenType.RBrace); }

    private Token Match(TokenType expected) 
    {
        if (index < tokens.Count && tokens[index].Type == expected) 
        {
            return tokens[index++];
        }
        throw new Exception($"Syntax Error: Expected {expected} but found {tokens[index].Type}");
    }
}
}
