# shadcn/ui Base UI Component Catalog

70+ components. All Base UI primitives import from `@base-ui/react`.
Install any component: `npx shadcn add [name]`

---

## Layout & Navigation

### Accordion
`import { Accordion } from '@base-ui/react/Accordion'`
Parts: Root, Item, Header, Trigger, Panel

### Breadcrumb
Composed from shadcn primitives (no Base UI dependency).
Parts: Root, List, Item, Link, Separator, Page, Ellipsis

### Collapsible
`import { Collapsible } from '@base-ui/react/Collapsible'`
Parts: Root, Trigger, Content

### Navigation Menu
`import { NavigationMenu } from '@base-ui/react/NavigationMenu'`
Parts: Root, List, Item, Trigger, Content, Link, Indicator, Viewport

### Pagination
Composed from Button. Parts: Root, Content, Item, Previous, Next, Link, Ellipsis

### Resizable
Uses `react-resizable-panels`. Parts: PanelGroup, Panel, Handle

### Scroll Area
`import { ScrollArea } from '@base-ui/react/ScrollArea'`
Parts: Root, Viewport, Scrollbar, Thumb, Corner

### Separator
`import { Separator } from '@base-ui/react/Separator'`

### Sidebar
Composed component. Parts: Provider, Root, Header, Content, Footer, Group, GroupLabel, Menu, MenuItem, Trigger, Rail, Inset

### Tabs
`import { Tabs } from '@base-ui/react/Tabs'`
Parts: Root, List, Trigger, Content, Indicator

---

## Data Display

### Aspect Ratio
CSS-based utility component

### Avatar
`import { Avatar } from '@base-ui/react/Avatar'`
Parts: Root, Image, Fallback

### Badge
Styled div with variants: default, secondary, destructive, outline

### Card
Composed from div. Parts: Root, Header, Title, Description, Content, Footer

### Carousel
Uses `embla-carousel-react`. Parts: Root, Content, Item, Previous, Next

### Chart
Uses `recharts`. Provides ChartContainer, ChartTooltip, ChartTooltipContent, ChartLegend, ChartLegendContent

### Data Table
Uses `@tanstack/react-table` with shadcn Table component

### Skeleton
CSS animation utility component

### Table
Composed from native `<table>`. Parts: Root, Header, Body, Footer, Row, Head, Cell, Caption

---

## Data Input

### Button
Styled button with variants: default, destructive, outline, secondary, ghost, link.
Sizes: default, sm, lg, icon

### Button Group *(new Oct 2025)*
Groups related buttons. Parts: Root, Item. Supports split button pattern

### Checkbox
`import { Checkbox } from '@base-ui/react/Checkbox'`
Parts: Root, Indicator

### Combobox
`import { Combobox } from '@base-ui/react/Combobox'`
Parts: Root, Trigger, Input, Portal, Popup, Listbox, Option, OptionText, OptionIndicator, Arrow, Empty

### Date Picker
Composed from Calendar + Popover. Uses `react-day-picker`

### Field *(new Oct 2025)*
`import { Field } from '@base-ui/react/Field'`
Works with React Hook Form, TanStack Form, Server Actions.
Parts: Root, Label, Control, Description, Error, Validity

### Input
Styled native `<input>` element

### Input Group *(new Oct 2025)*
Input with icons, buttons, labels, addons. Parts: Root, Input, Addon, Icon

### Input OTP
Uses `input-otp`. Parts: Root, Group, Slot, Separator

### Label
`import { Label } from '@base-ui/react/Label'`

### Native Select
Styled native `<select>` element

### Radio Group
`import { RadioGroup } from '@base-ui/react/RadioGroup'`
`import { Radio } from '@base-ui/react/Radio'`
Parts: Root, Item, Indicator

### Select
`import { Select } from '@base-ui/react/Select'`
Parts: Root, Trigger, Value, Portal, Popup, Listbox, Option, OptionText, OptionIndicator, Group, GroupLabel, Arrow, ScrollUpArrow, ScrollDownArrow

### Slider
`import { Slider } from '@base-ui/react/Slider'`
Parts: Root, Track, Range, Thumb, Output

### Switch
`import { Switch } from '@base-ui/react/Switch'`
Parts: Root, Thumb

### Textarea
Styled native `<textarea>` element

### Toggle
`import { Toggle } from '@base-ui/react/Toggle'`

### Toggle Group
`import { ToggleGroup } from '@base-ui/react/ToggleGroup'`
Parts: Root, Item

---

## Feedback

### Alert
Styled div with variants: default, destructive.
Parts: Root, Title, Description

### Alert Dialog
`import { AlertDialog } from '@base-ui/react/AlertDialog'`
Parts: Root, Trigger, Portal, Backdrop, Popup, Title, Description, Close

### Empty *(new Oct 2025)*
Empty state display. Parts: Root, Media, Title, Description, Actions

### Kbd *(new Oct 2025)*
Keyboard shortcut display. Renders `<kbd>` with styling

### Progress
`import { Progress } from '@base-ui/react/Progress'`
Parts: Root, Track, Indicator

### Skeleton
CSS utility for loading placeholders

### Sonner (Toast)
Uses `sonner` library. `import { Toaster } from '@/components/ui/sonner'`
Call `toast()` / `toast.success()` / `toast.error()` etc

### Spinner *(new Oct 2025)*
Loading indicator with customizable designs

---

## Overlay

### Command
Uses `cmdk`. Parts: Root, Input, List, Empty, Group, Item, Separator, Shortcut

### Context Menu
`import { ContextMenu } from '@base-ui/react/ContextMenu'`
Parts: Root, Trigger, Portal, Popup, Group, Item, CheckboxItem, RadioGroup, RadioItem, Label, Separator, Arrow

### Dialog
`import { Dialog } from '@base-ui/react/Dialog'`
Parts: Root, Trigger, Portal, Backdrop, Popup, Title, Description, Close

### Drawer
Uses `vaul`. Parts: Root, Trigger, Portal, Overlay, Content, Header, Title, Description, Footer, Close

### Dropdown Menu
`import { Menu } from '@base-ui/react/Menu'`
Parts: Root, Trigger, Portal, Popup, Group, Item, CheckboxItem, RadioGroup, RadioItem, Label, Separator, Arrow

### Hover Card
`import { HoverCard } from '@base-ui/react/HoverCard'`
Parts: Root, Trigger, Portal, Popup, Arrow

### Menubar
Composed from Menu. Parts: Root, Menu, Trigger, Content, Item, Separator, Sub, SubTrigger, SubContent

### Popover
`import { Popover } from '@base-ui/react/Popover'`
Parts: Root, Trigger, Portal, Popup, Title, Description, Close, Arrow

### Sheet
Variant of Dialog positioned at screen edge. Parts: Root, Trigger, Portal, Backdrop, Content, Header, Title, Description, Footer, Close

### Tooltip
`import { Tooltip } from '@base-ui/react/Tooltip'`
Parts: Root, Trigger, Portal, Popup, Arrow

---

## Typography & Utilities

### Direction
RTL/LTR direction provider

### Item *(new Oct 2025)*
Flexible list/card item. Parts: Root, Content, Title, Description, Media, Actions

### Typography
Prose styling utilities for headings, paragraphs, lists, etc
