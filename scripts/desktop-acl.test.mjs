import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

// Check the manifests emitted by tauri-build, not a mocked invoke() call.
// Run a Cargo check/build before this test to refresh the generated ACL.
const manifests = JSON.parse(
  readFileSync(new URL('../src-tauri/gen/schemas/acl-manifests.json', import.meta.url), 'utf8')
);
const capabilities = JSON.parse(
  readFileSync(new URL('../src-tauri/gen/schemas/capabilities.json', import.meta.url), 'utf8')
);
const desktop = capabilities['desktop-commands'];
const appPermissions = manifests['__app-acl__'].permissions;

test('the release window can invoke the actual desktop notification command', () => {
  assert.ok(desktop.permissions.includes('allow-show-desktop-notification'));
  assert.deepEqual(
    appPermissions['allow-show-desktop-notification'].commands.allow,
    ['show_desktop_notification']
  );
  assert.deepEqual(appPermissions['allow-show-desktop-notification'].commands.deny, []);
});

test('enabling the app manifest preserves push-to-talk command access', () => {
  assert.ok(desktop.permissions.includes('allow-set-push-to-talk-binds'));
  assert.deepEqual(
    appPermissions['allow-set-push-to-talk-binds'].commands.allow,
    ['set_push_to_talk_binds']
  );
});

test('the release window can read and save close behavior', () => {
  for (const command of ['get_quit_on_close', 'set_quit_on_close']) {
    const permission = `allow-${command.replaceAll('_', '-')}`;
    assert.ok(desktop.permissions.includes(permission));
    assert.deepEqual(appPermissions[permission].commands.allow, [command]);
  }
});

test('custom commands are granted only to the main window and Cinny release origin', () => {
  assert.equal(desktop.local, true);
  assert.deepEqual(desktop.windows, ['main']);
  assert.deepEqual(desktop.webviews ?? [], []);
  assert.deepEqual(desktop.remote.urls, ['http://localhost:44548/*']);
  assert.deepEqual(new Set(desktop.platforms), new Set(['windows', 'macOS', 'linux']));
});
