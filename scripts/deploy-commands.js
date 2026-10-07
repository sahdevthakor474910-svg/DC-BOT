/**
 * Dedicated Slash Command Deployment Script
 * 
 * Usage:
 *   node scripts/deploy-commands.js
 * 
 * This script is separated from normal bot startup so that the bot process
 * NEVER spam-registers slash commands on every container restart.
 * 
 * Commands only need to be registered ONCE when they are created or modified.
 */

const fs = require('fs');
const path = require('path');

// Simple .env parser to avoid external dependencies
function loadEnv() {
    const envPath = path.resolve(__dirname, '../.env');
    if (!fs.existsSync(envPath)) return;
    const lines = fs.readFileSync(envPath, 'utf8').split('\n');
    for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;
        const eqIdx = trimmed.indexOf('=');
        if (eqIdx !== -1) {
            const key = trimmed.slice(0, eqIdx).trim();
            const val = trimmed.slice(eqIdx + 1).trim().replace(/^["']|["']$/g, '');
            if (!process.env[key]) {
                process.env[key] = val;
            }
        }
    }
}

loadEnv();

const token = process.env.DISCORD_TOKEN;
const clientId = process.env.DISCORD_CLIENT_ID;

console.log('==============================================');
console.log('🤖 Discord Bot Slash Command Deployment Tool');
console.log('==============================================');

if (!token || !clientId) {
    console.error('❌ Missing DISCORD_TOKEN or DISCORD_CLIENT_ID in environment / .env');
    process.exit(1);
}

console.log(`Client ID: ${clientId}`);
console.log(`Token: ${token.slice(0, 10)}...`);

console.log('\n[INFO] Command Deployment Architecture:');
console.log('1. Normal bot startup on Render does NOT register commands automatically (zero startup REST requests).');
console.log('2. Discord retains previously registered slash commands permanently.');
console.log('3. To deploy or update slash commands on your server, you can:');
console.log('   a) Set DEPLOY_COMMANDS=true in your Render environment variables and restart once.');
console.log('   b) Run "/admin deploy-commands" directly inside Discord in your server.');
console.log('   c) Pass --deploy-commands flag when running the dc-bot binary.');
console.log('==============================================\n');
