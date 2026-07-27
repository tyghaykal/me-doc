// Starter snippets for the toolbar's "New diagram type" picker. Keyed by a
// short label; `source` is inserted when the user picks a type.
export interface DiagramTemplate {
  label: string
  icon: string
  source: string
}

export const DIAGRAM_TEMPLATES: DiagramTemplate[] = [
  {
    label: 'Flowchart',
    icon: '⤵',
    source: 'graph TD\n  A[Start] --> B{Decision}\n  B -->|Yes| C[Do this]\n  B -->|No| D[Do that]\n',
  },
  {
    label: 'Sequence',
    icon: '⇄',
    source:
      'sequenceDiagram\n  participant A as Alice\n  participant B as Bob\n  A->>B: Hello Bob\n  B-->>A: Hi Alice\n',
  },
  {
    label: 'Class',
    icon: '▤',
    source:
      'classDiagram\n  class Animal {\n    +String name\n    +move()\n  }\n  Animal <|-- Dog\n',
  },
  {
    label: 'State',
    icon: '◉',
    source: 'stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running: start\n  Running --> [*]: stop\n',
  },
  {
    label: 'ER',
    icon: '⌗',
    source:
      'erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER ||--|{ LINE_ITEM : contains\n',
  },
  {
    label: 'Gantt',
    icon: '▬',
    source:
      'gantt\n  title Project plan\n  dateFormat YYYY-MM-DD\n  section Phase 1\n  Design :a1, 2026-01-01, 7d\n  Build  :after a1, 10d\n',
  },
  {
    label: 'Pie',
    icon: '◔',
    source: 'pie title Share\n  "A" : 40\n  "B" : 35\n  "C" : 25\n',
  },
  {
    label: 'Mindmap',
    icon: '❖',
    source: 'mindmap\n  root((Idea))\n    Branch A\n      Leaf 1\n    Branch B\n',
  },
]

export const DEFAULT_DIAGRAM_SOURCE = DIAGRAM_TEMPLATES[0]!.source
