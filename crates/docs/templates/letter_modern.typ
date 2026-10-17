// Waypoint "Modern" cover letter. Same header and indigo accent as the Modern resume.
#let d = json("/data.json")
#let accent = rgb("#4a57e8")

#set document(title: d.name + " — Cover letter", author: d.name)
#set page(paper: if d.at("paper", default: "a4") == "us-letter" { "us-letter" } else { "a4" }, margin: (x: 2.1cm, top: 1.4cm, bottom: 1.8cm))
#set text(font: "Inter", size: 10.3pt, fill: rgb("#1d1d1f"), lang: d.at("lang", default: "en"))
#set par(leading: 0.68em, spacing: 1.05em)
#set list(indent: 0.2em, body-indent: 0.6em, spacing: 0.7em, marker: text(fill: accent)[▸])

#let muted(body) = text(fill: rgb("#6e6e73"), body)

#grid(columns: (1fr, auto), align: bottom,
  text(size: 22pt, weight: "bold", fill: accent, tracking: -0.015em, d.name),
  align(right, muted(text(size: 9.2pt, d.contact.join(linebreak())))))
#v(-0.3em)
#line(length: 100%, stroke: 0.6pt + accent.lighten(65%))
#v(1.2em)

#grid(columns: (1fr, auto), gutter: 1em,
  d.recipient.join(linebreak()),
  text(fill: accent, weight: "medium", d.date))
#v(1.4em)

#if d.subject != "" { text(size: 11pt, weight: "bold", fill: accent, d.subject); v(0.3em) }
#if d.salutation != "" { d.salutation }

#if d.opening != "" { d.opening }

#if d.highlights.len() > 0 { list(..d.highlights) }

#for p in d.body { par(p) }

#v(0.6em)
#d.closing
#v(1.6em)
#text(weight: "semibold", d.signature)
