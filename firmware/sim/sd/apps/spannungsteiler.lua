-- name: Voltage divider
-- name.de: Spannungsteiler
-- kind: tool
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: The output voltage and the current of a voltage divider.
-- about.de: Ausgangsspannung und Strom eines Spannungsteilers.
-- license: MIT
-- tile: yes

-- A tool that only asks for values and shows results: a form, with no callbacks. The units
-- are the ones a number typed alone is taken in; 10 kΩ typed into R2 is taken as typed.

local de = lang() == "de"

form({
  title = de and "Spannungsteiler" or "Voltage divider",
  fields = {
    {id = "U",  label = de and "Eingang" or "Input", unit = "V"},
    {id = "R1", label = "R1", unit = "Ω"},
    {id = "R2", label = "R2", unit = "Ω", value = "10 kΩ", hint = de and "Der Widerstand am Ausgang" or "The resistor at the output"},
  },
  results = {
    {label = "U2", formula = "U*R2/(R1+R2)", unit = "V"},
    {label = "I",  formula = "U/(R1+R2)",    unit = "mA"},
  },
})
