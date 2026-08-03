package app.tobiasschmidt.quantus

import org.springframework.boot.SpringApplication
import org.springframework.boot.autoconfigure.SpringBootApplication

@SpringBootApplication
class QuantusApplication

fun main(args: Array<String>) {
    SpringApplication.run(QuantusApplication::class.java, *args)
}
