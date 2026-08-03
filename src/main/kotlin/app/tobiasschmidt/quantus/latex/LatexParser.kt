package app.tobiasschmidt.quantus.latex

import app.tobiasschmidt.quantus.cas.Constants
import app.tobiasschmidt.quantus.units.UnitLookup
import java.math.BigDecimal

/**
 * Recursive-descent parser for the LaTeX subset from F2.
 *
 * Grammar (each level binds tighter than the one above):
 * ```
 * formula     := [ label '=' ] expression
 * expression  := term ( ('+' | '-') term )*
 * term        := unary ( ('\cdot' | '\times' | '*' | '/' | juxtaposition) unary )*
 * unary       := ('-' | '+') unary | power
 * power       := atom [ '^' unary ]
 * atom        := number | identifier | '\frac' arg arg | '\sqrt' ['[' expr ']'] arg
 *              | '\mathrm' arg | '\text' arg | '\qty' arg arg | '(' expression ')' | '{' expression '}'
 * ```
 *
 * The interesting part is how bare identifiers are classified — the uniqueness rule from F2:
 * 1. identifier **with** a subscript → variable (`v_1`, `\omega_0`),
 * 2. otherwise the longest run of adjacent identifiers that resolves against the unit table
 *    → unit (`kg`, `mN`, `\mu m` → µm),
 * 3. otherwise the constant table (`\pi`, `e`, `i`),
 * 4. otherwise a parse error naming the position.
 *
 * Step 2 is deliberately greedy, which is what makes implicit unit products illegal: `m s`
 * resolves to millisecond, not metre·second. Products of units need an explicit `\cdot`.
 */
class LatexParser(private val source: String) {

    private val tokens: List<Token> = LatexLexer(source).tokenize()
    private var index = 0

    /** Inside `\mathrm{}`/`\text{}`/`\qty{}{}` every identifier must be a unit. */
    private var unitContext = false

    fun parse(): Formula {
        if (tokens.size == 1) {
            throw LatexParseException("The input is empty.", 0, source)
        }
        val label = parseLabel()
        val expression = parseExpression()
        if (current.type == TokenType.EQUALS) {
            throw LatexParseException("Only one '=' is allowed, and only right after the label.", current.position, source)
        }
        expect(TokenType.END, "the end of the input")
        return Formula(label, FractionNormalizer.normalize(expression), source)
    }

    // ---- structure -------------------------------------------------------------------

    /** `F =` or `r_1 =`; anything else means the input is a bare expression. */
    private fun parseLabel(): Label? {
        val start = index
        if (current.type != TokenType.IDENTIFIER) return null

        val name = advance()
        val subscript = if (current.type == TokenType.UNDERSCORE) {
            advance()
            parseSubscript()
        } else {
            null
        }
        if (current.type != TokenType.EQUALS) {
            index = start
            return null
        }
        advance()
        return Label(name.text, subscript, name.position)
    }

    private fun parseExpression(): Node {
        var left = parseTerm()
        while (current.type == TokenType.PLUS || current.type == TokenType.MINUS) {
            val operator = if (advance().type == TokenType.PLUS) Operator.PLUS else Operator.MINUS
            left = BinaryNode(operator, left, parseTerm(), left.position)
        }
        return left
    }

    private fun parseTerm(): Node {
        var left = parseUnary()
        while (true) {
            val operator = when {
                current.type == TokenType.TIMES -> Operator.TIMES
                current.type == TokenType.DIVIDE -> Operator.DIVIDE
                current.type == TokenType.COMMAND && current.text in MULTIPLICATION_COMMANDS -> Operator.TIMES
                current.type == TokenType.COMMAND && current.text == "\\div" -> Operator.DIVIDE
                startsAtom() -> Operator.IMPLICIT_TIMES
                else -> return left
            }
            if (operator != Operator.IMPLICIT_TIMES) advance()
            left = BinaryNode(operator, left, parseUnary(), left.position)
        }
    }

    private fun parseUnary(): Node = when (current.type) {
        TokenType.MINUS -> {
            val token = advance()
            NegationNode(parseUnary(), token.position)
        }

        TokenType.PLUS -> {
            advance()
            parseUnary()
        }

        else -> parsePower()
    }

    private fun parsePower(): Node {
        val base = parseAtom()
        if (current.type != TokenType.CARET) return base
        advance()
        return PowerNode(base, parseUnary(), base.position)
    }

    private fun parseAtom(): Node {
        val token = current
        return when {
            token.type == TokenType.NUMBER -> {
                advance()
                NumberNode(BigDecimal(token.text), token.position)
            }

            token.type == TokenType.IDENTIFIER -> parseIdentifier()
            token.type == TokenType.LPAREN -> parseGroup(TokenType.RPAREN, ")")
            token.type == TokenType.LBRACE -> parseGroup(TokenType.RBRACE, "}")
            token.type == TokenType.COMMAND -> parseCommand(token)
            else -> throw LatexParseException("Expected a number, a unit or a variable here.", token.position, source)
        }
    }

    private fun parseGroup(closing: TokenType, closingText: String): Node {
        val open = advance()
        val inner = parseExpression()
        expect(closing, "'$closingText'")
        return GroupNode(inner, open.position)
    }

    private fun parseCommand(token: Token): Node = when (token.text) {
        "\\frac", "\\dfrac", "\\tfrac" -> {
            advance()
            val numerator = parseArgument(token.text)
            val denominator = parseArgument(token.text)
            FractionNode(numerator, denominator, token.position)
        }

        "\\sqrt" -> {
            advance()
            val degree = if (current.type == TokenType.LBRACKET) {
                advance()
                parseExpression().also { expect(TokenType.RBRACKET, "']'") }
            } else {
                null
            }
            RootNode(parseArgument(token.text), degree, token.position)
        }

        // Explicit unit markup: \mathrm{kg}, \text{m/s^2}, siunitx' \si{} and \unit{}.
        "\\mathrm", "\\text", "\\si", "\\unit" -> {
            advance()
            inUnitContext { parseArgument(token.text) }
        }

        // siunitx: \qty{12}{cm} is a number and a unit, i.e. an implicit product.
        "\\qty", "\\SI", "\\num" -> {
            advance()
            val value = parseArgument(token.text)
            if (token.text == "\\num") {
                value
            } else {
                BinaryNode(Operator.IMPLICIT_TIMES, value, inUnitContext { parseArgument(token.text) }, token.position)
            }
        }

        else -> throw LatexParseException("Unsupported command '${token.text}'.", token.position, source)
    }

    /** A command argument: `{…}` — braces are mandatory, which keeps `\frac12` from surprising anyone. */
    private fun parseArgument(command: String): Node {
        if (current.type != TokenType.LBRACE) {
            throw LatexParseException("'$command' expects its argument in braces, e.g. $command{…}.", current.position, source)
        }
        return parseGroup(TokenType.RBRACE, "}")
    }

    private fun <T> inUnitContext(block: () -> T): T {
        val previous = unitContext
        unitContext = true
        try {
            return block()
        } finally {
            unitContext = previous
        }
    }

    // ---- identifiers: the uniqueness rule from F2 ------------------------------------

    private fun parseIdentifier(): Node {
        val token = current
        if (!unitContext && peek(1).type == TokenType.UNDERSCORE) {
            advance()
            advance()
            return VariableNode(token.text, parseSubscript(), token.position)
        }

        // The whole run has to resolve as one symbol: `m s` is the millisecond, and a
        // product of units is only ever written with an explicit \cdot.
        val run = identifierRun()
        val symbol = run.joinToString("") { symbolOf(it) }
        val unit = UnitLookup.find(symbol)
        if (unit != null) {
            index += run.size
            return UnitNode(unit, symbol, token.position)
        }

        if (!unitContext && run.size == 1 && Constants.isConstant(token.text)) {
            advance()
            return ConstantNode(token.text, token.position)
        }

        throw LatexParseException(unknownIdentifierMessage(symbol, run.size), token.position, source)
    }

    private fun unknownIdentifierMessage(symbol: String, runLength: Int): String {
        val hint = if (runLength > 1) " A product of units needs an explicit \\cdot." else ""
        return when {
            unitContext -> "'$symbol' is not a known unit.$hint"
            else ->
                "'$symbol' is neither a unit nor a constant. Variables must carry a subscript, e.g. v_1.$hint"
        }
    }

    /**
     * The adjacent identifiers eligible for the longest-match lookup. An identifier carrying
     * a subscript belongs to a variable and ends the run before it.
     */
    private fun identifierRun(): List<Token> {
        val run = mutableListOf<Token>()
        var offset = 0
        while (peek(offset).type == TokenType.IDENTIFIER && peek(offset + 1).type != TokenType.UNDERSCORE) {
            run += peek(offset)
            offset++
        }
        return run
    }

    /** `\mu` is the µ of `µm`, `\Omega` the Ω of `k\Omega`. */
    private fun symbolOf(token: Token): String = GREEK_SYMBOLS[token.text] ?: token.text

    /** The index of a variable: `r_1`, `r_{12}`, `x_{ges}`. */
    private fun parseSubscript(): String {
        if (current.type != TokenType.LBRACE) {
            val token = current
            if (token.type != TokenType.NUMBER && token.type != TokenType.IDENTIFIER) {
                throw LatexParseException("Expected an index after '_'.", token.position, source)
            }
            advance()
            return token.text
        }
        advance()
        val index = StringBuilder()
        while (current.type != TokenType.RBRACE) {
            if (current.type == TokenType.END) {
                throw LatexParseException("The index is missing its closing '}'.", current.position, source)
            }
            index.append(advance().text)
        }
        advance()
        if (index.isEmpty()) {
            throw LatexParseException("The index must not be empty.", current.position, source)
        }
        return index.toString()
    }

    // ---- token handling ---------------------------------------------------------------

    private val current: Token get() = tokens[index]

    private fun peek(offset: Int): Token = tokens[(index + offset).coerceAtMost(tokens.size - 1)]

    private fun advance(): Token = tokens[index++]

    private fun expect(type: TokenType, description: String) {
        if (current.type != type) {
            throw LatexParseException("Expected $description here.", current.position, source)
        }
        if (type != TokenType.END) advance()
    }

    /** Does the current token start an atom, i.e. is this juxtaposition (`2kg`)? */
    private fun startsAtom(): Boolean = when (current.type) {
        TokenType.NUMBER, TokenType.IDENTIFIER, TokenType.LPAREN, TokenType.LBRACE -> true
        TokenType.COMMAND -> current.text !in MULTIPLICATION_COMMANDS && current.text != "\\div"
        else -> false
    }

    private companion object {
        val MULTIPLICATION_COMMANDS = setOf("\\cdot", "\\times", "\\ast")

        val GREEK_SYMBOLS = mapOf("\\mu" to "µ", "\\Omega" to "Ω", "\\omega" to "ω")
    }
}
