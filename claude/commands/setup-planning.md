---
description: Create planning workflow files and maintain them automatically
argument-hint: [what to plan]
---

You are setting up a planning workflow.

Planning topic: **$ARGUMENTS**

**First, check if a planning topic was provided:**

- If the line above shows "Planning topic: \*\*\*\*" (empty), simply ask the user what they want to plan and wait for their response
- Once you have the planning topic (either from arguments or from asking), proceed with the workflow below

Follow these steps:

1. **Create the directory structure:**
   - Create `temp_docs/` directory in the current working directory if it doesn't exist

2. **Research the codebase:**
   - Use appropriate tools (Grep, Glob, Read) to understand the codebase related to the planning topic
   - Identify relevant files, patterns, and existing implementations
   - Note any dependencies, constraints, or related functionality
   - Use WebSearch to find relevant documentation, blogs, or forums for further research if needed

3. **Create and populate the planning files:**

   **temp_docs/plan.md:**
   - Write a comprehensive implementation plan
   - Include: overview, approach, detailed steps, considerations, potential challenges
   - Structure it clearly with markdown headings
   - Base it on your codebase research
   - Be thorough and detailed, include specifics and edge cases
   - If you did WebSearch research tell me what you found with relevant links

   **temp_docs/plan_progress.md:**
   - Initialize with a progress tracking structure
   - List the main tasks/steps from the plan
   - Mark all as pending initially
   - Include a format that's easy to update (e.g., checkboxes or status indicators)

   **temp_docs/plan_context.md:**
   - Initialize with sections for:
     - Important learnings and context from codebase research
     - Known blockers or challenges
     - Unanswered questions that need resolution
   - Populate with initial findings from your research

4. **Ongoing maintenance (IMPORTANT):**
   Throughout this conversation, automatically maintain these files:
   - Update `plan_progress.md` as tasks are started and completed
   - Add new learnings, blockers, suprising findings, and questions to `plan_context.md`
   - Update `plan.md` if the approach or steps change
   - Do this proactively without being asked

5. **Confirm:**
   After creating the files, provide a brief summary of the plan and confirm the planning workflow is active.
