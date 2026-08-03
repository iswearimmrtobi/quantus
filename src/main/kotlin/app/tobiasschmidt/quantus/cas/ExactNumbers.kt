package app.tobiasschmidt.quantus.cas

import org.matheclipse.core.expression.F
import org.matheclipse.core.interfaces.IExpr
import java.math.BigDecimal
import java.math.BigInteger

/**
 * Conversion of JVM numbers into *exact* Symja expressions.
 *
 * Background (phase 0, spike 1): Symja turns decimal literals into machine doubles —
 * `N(Sqrt(2.0), 30)` is capped at 16 digits. Every number entering the system is therefore
 * stored as a rational: `9.81` becomes `981/100`. Only at the output boundary does
 * [CasEngine.decimal] turn it back into a decimal — then with the full digit count.
 */
fun exactExpr(value: BigDecimal): IExpr {
    val scale = value.scale()
    return if (scale <= 0) {
        F.ZZ(value.toBigIntegerExact())
    } else {
        F.QQ(value.unscaledValue(), BigInteger.TEN.pow(scale))
    }
}

fun exactExpr(value: Number): IExpr = when (value) {
    is BigDecimal -> exactExpr(value)
    is BigInteger -> F.ZZ(value)
    is Int, is Long, is Short, is Byte -> F.ZZ(value.toLong())
    // A double reaching us comes from library metadata (Indriya's 273.15 offset for degree
    // celsius, say) and means the decimal literal it was written as — not its exact binary
    // expansion. `toString()` gives back exactly that literal: 273.15 stays 5463/20 instead
    // of becoming 2402652809016115/8796093022208.
    is Double -> exactExpr(BigDecimal(value.toString()))
    is Float -> exactExpr(BigDecimal(value.toString()))
    else -> exactExpr(BigDecimal(value.toString()))
}
