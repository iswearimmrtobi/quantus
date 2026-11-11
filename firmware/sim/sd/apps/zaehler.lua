-- name: Counter
-- name.de: Zähler
-- kind: tool
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Counts up and down with the arrows.
-- about.de: Zählt mit den Pfeilen auf und ab.
-- license: MIT

local n = 0

function key(k)
  if k == "up" then n = n + 1 end
  if k == "down" then n = n - 1 end
end

function draw()
  clear("background")
  local label = lang() == "de" and "Zähler" or "Count"
  text(20, 40, label .. ": " .. n, "text")
end
