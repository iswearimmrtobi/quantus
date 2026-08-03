package app.tobiasschmidt.quantus.units

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.cas.exactExpr
import org.matheclipse.core.expression.F
import org.matheclipse.core.interfaces.IExpr
import tech.units.indriya.function.AddConverter
import tech.units.indriya.function.MultiplyConverter
import tech.units.indriya.function.PowerOfIntConverter
import tech.units.indriya.function.RationalConverter
import tech.units.indriya.function.RationalNumber
import javax.measure.Unit
import javax.measure.UnitConverter

/**
 * A unit conversion as an exact affine map `y = factor · x + offset`.
 *
 * Purely multiplicative conversions (the common case) have `offset = 0`; degree celsius to
 * kelvin has `factor = 1, offset = 273.15`.
 */
data class AffineConversion(val factor: IExpr, val offset: IExpr) {

    val isIdentity: Boolean get() = factor.isOne && offset.isZero

    fun apply(value: IExpr): IExpr = when {
        isIdentity -> value
        offset.isZero -> F.eval(F.Times(factor, value))
        else -> F.eval(F.Plus(F.Times(factor, value), offset))
    }
}

/**
 * Decomposes Indriya conversions into exact Symja factors.
 *
 * Why not simply `UnitConverter.convert(BigDecimal)`? Because Indriya computes with
 * `MathContext.DECIMAL128`: `km/h → m/s` (factor 5/18) would be cut off after 34 digits and
 * F3 (50 digits and beyond) lost at the conversion. The conversion steps do carry the exact
 * factor as a fraction though — it moves into the Symja expression unchanged.
 */
object UnitConversions {

    /** Exact conversion from [from] to [to]. Both units must be compatible. */
    fun between(from: Unit<*>, to: Unit<*>): AffineConversion {
        if (from == to) return IDENTITY
        return fold(from.getConverterToAny(to))
    }

    private val IDENTITY = AffineConversion(F.C1, F.C0)

    /**
     * Folds the steps of a (possibly composed) conversion into a single affine map.
     *
     * `getConversionSteps()` lists the steps in **reverse** application order: a composed
     * converter is a `Pair` with `convert(x) = left(right(x))`, while its step list reads
     * `[left…, right…]`. Hence the iteration over the reversed list — otherwise `2·°C → K`
     * would come out as `2x + 546.3` instead of `2x + 273.15`.
     *
     * A multiplication step scales the offset collected so far, an addition step does not
     * touch the factor.
     */
    private fun fold(converter: UnitConverter): AffineConversion {
        var factor: IExpr = F.C1
        var offset: IExpr = F.C0
        for (step in converter.conversionSteps.reversed()) {
            when {
                step.isIdentity -> {}
                step is AddConverter -> offset = F.eval(F.Plus(offset, numberExpr(step.offset)))
                step is MultiplyConverter -> {
                    val stepFactor = factorOf(step)
                    factor = F.eval(F.Times(factor, stepFactor))
                    offset = F.eval(F.Times(offset, stepFactor))
                }

                else -> throw UnsupportedConversionException(
                    "This unit conversion is not supported (${step.javaClass.simpleName}): " +
                        "only linear and affine conversions work, not logarithmic ones.",
                )
            }
        }
        return AffineConversion(factor, offset)
    }

    private fun factorOf(step: MultiplyConverter): IExpr = when {
        step is RationalConverter -> F.QQ(step.dividend, step.divisor)
        // Prefixes (kilo, milli, …) live in here as powers of ten.
        step is PowerOfIntConverter -> step.toRationalConverter().let { F.QQ(it.dividend, it.divisor) }
        // Keep powers of π (degree → radian) exact instead of collecting a double
        // approximation. PowerOfPiConverter is package-private; the name is all we have.
        step.javaClass.simpleName == PI_CONVERTER -> F.Power(F.Pi, F.ZZ(piExponent(step)))
        else -> numberExpr(step.factor)
    }

    private const val PI_CONVERTER = "PowerOfPiConverter"

    private fun piExponent(step: MultiplyConverter): Long =
        (step as tech.uom.lib.common.function.IntExponentSupplier).exponent.toLong()

    private fun numberExpr(value: Number): IExpr = when (value) {
        is RationalNumber -> F.QQ(value.dividend, value.divisor)
        else -> exactExpr(value)
    }
}

/** A conversion that cannot be expressed as `factor · x + offset` (e.g. decibel). */
class UnsupportedConversionException(message: String) : QuantusException(message)
