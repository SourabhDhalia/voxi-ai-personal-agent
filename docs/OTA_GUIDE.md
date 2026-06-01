# Voxi OTA Updates Guide

This guide explains how to check for, execute, and rollback Over-the-Air (OTA) updates for Voxi's modular agent skills using the dashboard interface and API endpoints.

---

## 1. Overview

Voxi supports updating individual textual and capability skills dynamically without needing a full daemon rebuild or service restart. This allows operators to push behavior improvements, bug fixes, or new prompts to the agent's active capability catalog.

### Core Capabilities:
- **Check for Updates**: Compares local skill definitions against remote repositories or manifests.
- **Over-the-Air Update**: Downloads and updates the selected skill's `SKILL.md` or directory contents.
- **Version Rollback**: Restores the previously backed-up version of a skill in case of regression or configuration failure.

---

## 2. Operating via Web Dashboard

The easiest way to manage OTA updates is through the Voxi Web Dashboard interface.

### Step-by-Step Flow:
1. Open the Web Dashboard in your browser (default host port: `9091`).
2. Navigate to the **OTA Updates** tab in the sidebar navigation menu.
3. Click the **Check for Updates** button to trigger a scan.
4. **Inspect the Status & Manifest**:
   - The status bar will show the scan result (e.g., "All skills up to date" or a count of available updates).
   - Skills will list their local version alongside the latest remote version, decorated with a badge:
     - <span style="background-color:#ffe0b2;color:#b78103;padding:2px 6px;border-radius:4px;font-size:12px;">Update</span>: A newer version is available.
     - <span style="background-color:#c8e6c9;color:#256029;padding:2px 6px;border-radius:4px;font-size:12px;">Current</span>: The local skill matches the latest remote definition.
5. **Apply Updates**:
   - For any skill with an available update, click the **Update** button on the skill's card. The dashboard will trigger the download and update process, reloading the skill catalog on completion.
6. **Rollback Changes**:
   - If a skill behaves unexpectedly after an update, click the **Rollback** button on its card. This will immediately revert the skill to its previous cached/backed-up state.

---

## 3. API Reference

For headless automation or integrations, Voxi provides authenticated REST endpoints in the web dashboard binary.

> [!NOTE]
> All write/mutation endpoints require standard token authorization.

### 1. Check for Updates
- **Endpoint**: `/api/ota/check`
- **Method**: `GET`
- **Headers**: `Authorization: Bearer <token>`
- **Response**:
  ```json
  {
    "available_count": 0,
    "updates": []
  }
  ```

### 2. Update a Skill
- **Endpoint**: `/api/ota/update`
- **Method**: `POST`
- **Headers**: `Authorization: Bearer <token>`, `Content-Type: application/json`
- **Payload**:
  ```json
  {
    "skill": "shopping-assistant"
  }
  ```
- **Response**:
  ```json
  {
    "status": "up_to_date",
    "skill": "shopping-assistant"
  }
  ```

### 3. Rollback a Skill
- **Endpoint**: `/api/ota/rollback`
- **Method**: `POST`
- **Headers**: `Authorization: Bearer <token>`, `Content-Type: application/json`
- **Payload**:
  ```json
  {
    "skill": "shopping-assistant"
  }
  ```
- **Response**:
  ```json
  {
    "status": "rolled_back",
    "restored_version": "1.0.0",
    "skill": "shopping-assistant"
  }
  ```

---

## 4. Gating & Precedence Policies

OTA Updates respect the target-environment security rules:
- **Project Gating**: When `enable_project_skills` is set to `false`, OTA updates target only user-level installed paths (`~/.voxi/workspace/skills/`). Repo-level paths (`.agents/skills/`) remain protected.
- **Rollback Safety**: Before writing any new skill definition, Voxi retains the previous version in a temporary cache directory. If compilation, schema parsing, or FFI checks fail, a rollback is executed automatically.
