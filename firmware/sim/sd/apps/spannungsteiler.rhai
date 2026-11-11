// name: Voltage divider
// name.de: Spannungsteiler
// kind: tool
// api: 1
// author: Quantus
// version: 1.0
// about: The output voltage and the current of a voltage divider.
// about.de: Ausgangsspannung und Strom eines Spannungsteilers.
// license: MIT

// A tool that only asks for values and shows results: a form, with no callbacks. The units
// are the ones a number typed alone is taken in; 10 kΩ typed into R2 is taken as typed.

form(#{
    title: if lang() == "de" { "Spannungsteiler" } else { "Voltage divider" },
    fields: [
        #{ id: "U",  label: if lang() == "de" { "Eingang" } else { "Input" }, unit: "V" },
        #{ id: "R1", label: "R1", unit: "Ω" },
        #{ id: "R2", label: "R2", unit: "Ω", value: "10 kΩ", hint: if lang() == "de" { "Der Widerstand am Ausgang" } else { "The resistor at the output" } },
    ],
    results: [
        #{ label: "U2", formula: "U*R2/(R1+R2)", unit: "V" },
        #{ label: "I",  formula: "U/(R1+R2)",    unit: "mA" },
    ],
})
