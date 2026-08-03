package app.tobiasschmidt.quantus.web

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.latex.LatexParseException
import org.slf4j.LoggerFactory
import org.springframework.http.HttpStatus
import org.springframework.http.ResponseEntity
import org.springframework.http.converter.HttpMessageNotReadableException
import org.springframework.web.bind.annotation.ExceptionHandler
import org.springframework.web.bind.annotation.RestControllerAdvice

/**
 * Turns exceptions into the [ErrorResponse] the UI shows.
 *
 * The split follows [QuantusException]: its message is written for the user and travels as
 * a 400, everything else is a defect of ours and only leaves a 500 behind.
 */
@RestControllerAdvice
class ApiExceptionHandler {

    private val log = LoggerFactory.getLogger(javaClass)

    /** A parse error knows where it happened — the UI can point at the spot (F2). */
    @ExceptionHandler(LatexParseException::class)
    fun parseError(exception: LatexParseException) =
        ResponseEntity.badRequest().body(ErrorResponse(exception.message ?: exception.reason, exception.position))

    @ExceptionHandler(QuantusException::class)
    fun domainError(exception: QuantusException) =
        ResponseEntity.badRequest().body(ErrorResponse(exception.message ?: "The input could not be evaluated."))

    @ExceptionHandler(HttpMessageNotReadableException::class)
    fun malformedRequest(exception: HttpMessageNotReadableException) =
        ResponseEntity.badRequest().body(ErrorResponse("The request body is not valid JSON for this endpoint."))

    @ExceptionHandler(Exception::class)
    fun unexpected(exception: Exception): ResponseEntity<ErrorResponse> {
        log.error("Unhandled error while serving a request", exception)
        return ResponseEntity.status(HttpStatus.INTERNAL_SERVER_ERROR)
            .body(ErrorResponse("Unexpected internal error: ${exception.javaClass.simpleName}."))
    }
}
