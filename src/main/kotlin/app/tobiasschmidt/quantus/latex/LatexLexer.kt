package app.tobiasschmidt.quantus.latex

import app.tobiasschmidt.quantus.QuantusException

enum class TokenType {
    NUMBER,

    /** A run of letters (`kg`) or a Greek-letter command (`\omega`). */
    IDENTIFIER,

    /** Any other backslash command (`\frac`, `\cdot`, `\sqrt`, …), text including the backslash. */
    COMMAND,
    PLUS,
    MINUS,
    TIMES,
    DIVIDE,
    CARET,
    UNDERSCORE,
    EQUALS,
    LBRACE,
    RBRACE,
    LPAREN,
    RPAREN,
    LBRACKET,
    RBRACKET,
    END,
}

data class Token(val type: TokenType, val text: String, val position: Int)

/**
 * Turns LaTeX source into tokens.
 *
 * Two things happen here that later stages rely on:
 * - **Numbers keep their decimal form as text.** `9{,}81`, `9,81` and `9.81` all become the
 *   token text `9.81`, which [java.math.BigDecimal] can read directly — the value never
 *   passes through a double.
 * - **Purely visual markup disappears.** Spacing (`\,`, `\quad`) and the delimiter commands
 *   `\left`/`\right` are dropped, so the parser only ever sees structure.
 */
class LatexLexer(private val source: String) {

    private var index = 0

    fun tokenize(): List<Token> {
        val tokens = mutableListOf<Token>()
        while (true) {
            skipWhitespace()
            if (index >= source.length) {
                tokens += Token(TokenType.END, "", index)
                return tokens
            }
            val char = source[index]
            when {
                char.isDigit() -> tokens += number()
                isIdentifierStart(char) -> tokens += identifier()
                char == '\\' -> command()?.let { tokens += it }
                else -> tokens += symbol()
            }
        }
    }

    private fun skipWhitespace() {
        while (index < source.length && (source[index].isWhitespace() || source[index] == '~' || source[index] == '&')) {
            index++
        }
    }

    private fun number(): Token {
        val start = index
        val digits = StringBuilder()
        while (index < source.length && source[index].isDigit()) digits.append(source[index++])

        val fraction = fractionalPart()
        if (fraction != null) digits.append('.').append(fraction)
        return Token(TokenType.NUMBER, digits.toString(), start)
    }

    /** Accepts `,5`, `.5` and the LaTeX-typeset German form `{,}5`. */
    private fun fractionalPart(): String? {
        val start = index
        if (index < source.length && (source[index] == ',' || source[index] == '.')) {
            index++
        } else if (index + 2 < source.length && source[index] == '{' &&
            (source[index + 1] == ',' || source[index + 1] == '.') && source[index + 2] == '}'
        ) {
            index += 3
        } else {
            return null
        }

        if (index >= source.length || !source[index].isDigit()) {
            index = start // a lone separator is not part of the number
            return null
        }
        val digits = StringBuilder()
        while (index < source.length && source[index].isDigit()) digits.append(source[index++])
        return digits.toString()
    }

    private fun identifier(): Token {
        val start = index
        if (source[index] == DEGREE) index++
        while (index < source.length && source[index].isLetter()) index++
        return Token(TokenType.IDENTIFIER, source.substring(start, index), start)
    }

    /** Returns `null` for commands that carry no meaning for us (spacing, delimiters). */
    private fun command(): Token? {
        val start = index
        index++ // the backslash
        if (index >= source.length) {
            throw LatexParseException("A backslash must be followed by a command name.", start, source)
        }

        if (!source[index].isLetter()) {
            val symbol = source[index++]
            if (symbol in SPACING_SYMBOLS) return null
            throw LatexParseException("Unknown command '\\$symbol'.", start, source)
        }

        while (index < source.length && source[index].isLetter()) index++
        val text = source.substring(start, index)
        return when {
            text in IGNORED_COMMANDS -> null
            text in GREEK_COMMANDS -> Token(TokenType.IDENTIFIER, text, start)
            else -> Token(TokenType.COMMAND, text, start)
        }
    }

    private fun symbol(): Token {
        val start = index
        val char = source[index++]
        val type = when (char) {
            '+' -> TokenType.PLUS
            '-' -> TokenType.MINUS
            '*', '·', '×' -> TokenType.TIMES
            '/' -> TokenType.DIVIDE
            '^' -> TokenType.CARET
            '_' -> TokenType.UNDERSCORE
            '=' -> TokenType.EQUALS
            '{' -> TokenType.LBRACE
            '}' -> TokenType.RBRACE
            '(' -> TokenType.LPAREN
            ')' -> TokenType.RPAREN
            '[' -> TokenType.LBRACKET
            ']' -> TokenType.RBRACKET
            else -> throw LatexParseException("Unexpected character '$char'.", start, source)
        }
        return Token(type, char.toString(), start)
    }

    private fun isIdentifierStart(char: Char): Boolean = char.isLetter() || char == DEGREE

    companion object {
        private const val DEGREE = '°'

        /** `\,` `\;` `\:` `\!` and friends: horizontal spacing, no meaning. */
        private val SPACING_SYMBOLS = setOf(',', ';', ':', '!', ' ', '\\')

        private val IGNORED_COMMANDS = setOf(
            "\\left", "\\right", "\\quad", "\\qquad", "\\displaystyle", "\\textstyle", "\\limits",
        )

        /** Greek letters are identifiers: variable names (`\omega_0`) or constants (`\pi`). */
        val GREEK_COMMANDS = setOf(
            "\\alpha", "\\beta", "\\gamma", "\\delta", "\\epsilon", "\\varepsilon", "\\zeta", "\\eta",
            "\\theta", "\\vartheta", "\\iota", "\\kappa", "\\lambda", "\\mu", "\\nu", "\\xi", "\\pi",
            "\\rho", "\\sigma", "\\tau", "\\upsilon", "\\phi", "\\varphi", "\\chi", "\\psi", "\\omega",
            "\\Gamma", "\\Delta", "\\Theta", "\\Lambda", "\\Xi", "\\Pi", "\\Sigma", "\\Upsilon",
            "\\Phi", "\\Psi", "\\Omega",
        )
    }
}

/**
 * A LaTeX input that cannot be parsed, with the position it went wrong at (F2).
 * [position] is a zero-based index into [source].
 */
class LatexParseException(
    val reason: String,
    val position: Int,
    val source: String,
) : QuantusException("$reason (at position ${position + 1}: '${excerpt(source, position)}')") {

    private companion object {
        /** A short window around the offending position, so the message is self-explanatory. */
        fun excerpt(source: String, position: Int): String {
            if (source.isEmpty()) return ""
            val start = position.coerceIn(0, source.length)
            val end = (start + 12).coerceAtMost(source.length)
            val snippet = source.substring(start, end)
            return if (end < source.length) "$snippet…" else snippet
        }
    }
}
