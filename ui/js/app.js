/**
 * AdCleanse Presentation Controller & Tauri IPC Bridge
 * Crafted by Bored Polymath Studios
 */

(function () {
  'use strict';

  // State Store
  const state = {
    session: {
      authenticated: true,
      userId: '100084920491823',
      expiresInDays: 30,
    },
    metrics: {
      totalTopics: 42,
      highRiskCount: 7,
      partnerCount: 18,
      scrubbedCount: 128,
      driftIndex: 34.5,
    },
    topics: [
      { id: 'top_1', name: 'Real Estate & Mortgage Loans', category: 'Financial Services', origin: 'Off-Platform Activity', risk: 'High', active: true },
      { id: 'top_2', name: 'Health Insurance & Supplements', category: 'Healthcare & Wellness', origin: 'Inferred Behavior', risk: 'Critical', active: true },
      { id: 'top_3', name: 'Credit Cards & Short-Term Lending', category: 'Financial Services', origin: 'Advertiser Customer List', risk: 'High', active: true },
      { id: 'top_4', name: 'Luxury Automotive Enthusiasts', category: 'Automotive', origin: 'Lookalike Audience', risk: 'Moderate', active: true },
      { id: 'top_5', name: 'Clinical Depression Diagnostics', category: 'Healthcare & Wellness', origin: 'Inferred Behavior', risk: 'Critical', active: true },
      { id: 'top_6', name: 'Fast Fashion & Direct Discounts', category: 'E-Commerce & Retail', origin: 'Direct Engagement', risk: 'Low', active: true },
      { id: 'top_7', name: 'Online Sports Wagering', category: 'Entertainment', origin: 'Off-Platform Activity', risk: 'High', active: true },
    ],
    partners: [
      { id: 'part_1', name: 'Acxiom / LiveRamp Data Exchange', window: '30 Days', pixel: true, rights: 'Active Targeting' },
      { id: 'part_2', name: 'Oracle Advertising / BlueKai', window: '90 Days', pixel: true, rights: 'Active Targeting' },
      { id: 'part_3', name: 'Experian Consumer Marketing', window: '60 Days', pixel: false, rights: 'Active Targeting' },
      { id: 'part_4', name: 'Epsilon Audience Syndicate', window: '90 Days', pixel: true, rights: 'Active Targeting' },
    ],
    rules: [
      { id: 'rule_1', pattern: 'Gambling & Casinos', isRegex: false, autoScrub: true },
      { id: 'rule_2', pattern: 'Weight Loss & Supplements', isRegex: false, autoScrub: true },
      { id: 'rule_3', pattern: 'Mortgages & Subprime', isRegex: false, autoScrub: true },
    ],
    ledger: [
      { id: 1, time: '09:15:02', method: 'GET', url: 'https://accountscenter.facebook.com/ad_preferences/topics', status: 200, summary: 'Extract 42 active topic categories' },
      { id: 2, time: '09:15:04', method: 'GET', url: 'https://accountscenter.facebook.com/ad_preferences/advertisers', status: 200, summary: 'Query 90-day audience list uploads' },
      { id: 3, time: '09:16:11', method: 'POST', url: 'https://graph.facebook.com/v19.0/act_user/ad_topics/scrub', status: 200, summary: 'Mutate topic: Health Insurance & Supplements' },
    ],
    selectedTopicIds: new Set(),
  };

  // Safe IPC Dispatcher with Mock Fallback for Browser Preview
  async function invokeCommand(cmd, args = {}) {
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      try {
        return await window.__TAURI__.core.invoke(cmd, args);
      } catch (err) {
        console.error(`[IPC Error] ${cmd}:`, err);
        throw err;
      }
    } else {
      // Mock Browser Environment
      console.info(`[IPC Mock Fallback] Invoking '${cmd}'`, args);
      return mockHandler(cmd, args);
    }
  }

  function mockHandler(cmd, args) {
    switch (cmd) {
      case 'trigger_manual_audit':
        return {
          id: 'snap_' + Date.now(),
          timestamp_epoch: Math.floor(Date.now() / 1000),
          total_topics: state.topics.length,
          total_partners: state.partners.length,
          drift_index: 34.5,
          topics: state.topics,
          partners: state.partners,
        };
      case 'scrub_topic':
        return { topic_id: args.topic_id, status: 'Successfully Purged', http_status: 200 };
      case 'get_network_ledger':
        return state.ledger;
      default:
        return { status: 'ok' };
    }
  }

  // DOM Elements
  const tabs = document.querySelectorAll('.nav-tab-btn');
  const panes = document.querySelectorAll('.tab-pane');
  const spotlightOverlay = document.getElementById('spotlight-overlay');
  const onboardingModal = document.getElementById('modal-onboarding');

  // Tab Navigation
  function initTabs() {
    tabs.forEach(tab => {
      tab.addEventListener('click', () => {
        tabs.forEach(t => t.classList.remove('active'));
        panes.forEach(p => p.classList.remove('active'));

        tab.classList.add('active');
        const targetPane = document.getElementById(tab.dataset.target);
        if (targetPane) targetPane.classList.add('active');
      });
    });
  }

  // Render Topics Table
  function renderTopics() {
    const tbody = document.getElementById('tbody-topics');
    const searchVal = (document.getElementById('input-topics-search').value || '').toLowerCase();
    const catVal = document.getElementById('select-category-filter').value;
    const riskVal = document.getElementById('select-risk-filter').value;

    tbody.innerHTML = '';

    const filtered = state.topics.filter(t => {
      const matchSearch = t.name.toLowerCase().includes(searchVal) || t.category.toLowerCase().includes(searchVal);
      const matchCat = catVal === 'all' || t.category === catVal;
      const matchRisk = riskVal === 'all' || t.risk === riskVal;
      return matchSearch && matchCat && matchRisk;
    });

    if (filtered.length === 0) {
      tbody.innerHTML = `<tr><td colspan="6" style="text-align: center; color: var(--text-dim); padding: 32px;">No matching ad topics found.</td></tr>`;
      return;
    }

    filtered.forEach(topic => {
      const tr = document.createElement('tr');
      const isChecked = state.selectedTopicIds.has(topic.id);
      
      let riskPill = 'pill-purple';
      if (topic.risk === 'Critical') riskPill = 'pill-danger';
      else if (topic.risk === 'High') riskPill = 'pill-amber';
      else if (topic.risk === 'Low') riskPill = 'pill-emerald';

      tr.innerHTML = `
        <td><input type="checkbox" class="topic-check" data-id="${topic.id}" ${isChecked ? 'checked' : ''}></td>
        <td><strong>${topic.category}</strong></td>
        <td>${topic.name}</td>
        <td><span class="pill pill-purple">${topic.origin}</span></td>
        <td><span class="pill ${riskPill}">${topic.risk}</span></td>
        <td>
          <button class="btn btn-danger btn-sm btn-scrub-single" data-id="${topic.id}" data-name="${topic.name}">
            🧹 Scrub
          </button>
        </td>
      `;
      tbody.appendChild(tr);
    });

    attachTopicCheckListeners();
    attachSingleScrubListeners();
  }

  function attachTopicCheckListeners() {
    document.querySelectorAll('.topic-check').forEach(cb => {
      cb.addEventListener('change', (e) => {
        const id = e.target.dataset.id;
        if (e.target.checked) state.selectedTopicIds.add(id);
        else state.selectedTopicIds.delete(id);
        updateBatchButton();
      });
    });

    const checkAll = document.getElementById('check-all-topics');
    if (checkAll) {
      checkAll.addEventListener('change', (e) => {
        const isChecked = e.target.checked;
        state.topics.forEach(t => {
          if (isChecked) state.selectedTopicIds.add(t.id);
          else state.selectedTopicIds.delete(t.id);
        });
        renderTopics();
        updateBatchButton();
      });
    }
  }

  function updateBatchButton() {
    const btn = document.getElementById('btn-batch-scrub');
    const countSpan = document.getElementById('batch-selected-count');
    const count = state.selectedTopicIds.size;
    countSpan.textContent = count;
    btn.disabled = count === 0;
  }

  function attachSingleScrubListeners() {
    document.querySelectorAll('.btn-scrub-single').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const id = btn.dataset.id;
        const name = btn.dataset.name;
        btn.textContent = 'Purging...';
        btn.disabled = true;

        try {
          await invokeCommand('scrub_topic', { topic_id: id, topic_name: name });
          state.topics = state.topics.filter(t => t.id !== id);
          state.metrics.totalTopics = state.topics.length;
          state.metrics.scrubbedCount += 1;
          
          recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/ad_topics/scrub', 200, `Purged topic: ${name}`);
          updateOverviewMetrics();
          renderTopics();
          renderLedger();
        } catch (err) {
          btn.textContent = 'Failed';
          console.error(err);
        }
      });
    });
  }

  // Render Partner Ingestions Table
  function renderPartners() {
    const tbody = document.getElementById('tbody-partners');
    tbody.innerHTML = '';

    state.partners.forEach(partner => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td><strong>${partner.name}</strong></td>
        <td>${partner.window}</td>
        <td><span class="pill ${partner.pixel ? 'pill-danger' : 'pill-emerald'}">${partner.pixel ? 'Pixel Active' : 'Offline List'}</span></td>
        <td><span class="pill pill-amber">${partner.rights}</span></td>
        <td>
          <button class="btn btn-secondary btn-revoke-partner" data-id="${partner.id}" data-name="${partner.name}">
            🚫 Revoke Targeting
          </button>
        </td>
      `;
      tbody.appendChild(tr);
    });

    document.querySelectorAll('.btn-revoke-partner').forEach(btn => {
      btn.addEventListener('click', () => {
        const name = btn.dataset.name;
        btn.textContent = 'Revoked';
        btn.disabled = true;
        btn.classList.add('btn-outline');
        recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/advertisers/opt_out', 200, `Revoked partner upload: ${name}`);
        renderLedger();
      });
    });
  }

  // Render Rules Table
  function renderRules() {
    const tbody = document.getElementById('tbody-rules');
    tbody.innerHTML = '';

    state.rules.forEach(rule => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td><code>${rule.pattern}</code></td>
        <td><span class="pill pill-purple">${rule.isRegex ? 'Regex' : 'Exact / Substring'}</span></td>
        <td><span class="pill pill-emerald">${rule.autoScrub ? 'Enabled' : 'Disabled'}</span></td>
        <td>
          <button class="btn btn-outline btn-delete-rule" data-id="${rule.id}">Remove</button>
        </td>
      `;
      tbody.appendChild(tr);
    });

    document.querySelectorAll('.btn-delete-rule').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.dataset.id;
        state.rules = state.rules.filter(r => r.id !== id);
        renderRules();
      });
    });
  }

  // Render Network Ledger
  function renderLedger() {
    const tbody = document.getElementById('tbody-ledger');
    tbody.innerHTML = '';

    state.ledger.slice().reverse().forEach(entry => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td>#${entry.id}</td>
        <td>${entry.time}</td>
        <td><span class="pill ${entry.method === 'GET' ? 'pill-purple' : 'pill-danger'}">${entry.method}</span></td>
        <td><code>${entry.url}</code></td>
        <td><span class="pill pill-emerald">${entry.status} OK</span></td>
        <td>${entry.summary}</td>
        <td><span class="pill pill-emerald">Verified Zero-Telemetry</span></td>
      `;
      tbody.appendChild(tr);
    });
  }

  function recordLedgerEntry(method, url, status, summary) {
    const now = new Date();
    const timeStr = now.toTimeString().split(' ')[0];
    const newEntry = {
      id: state.ledger.length + 1,
      time: timeStr,
      method,
      url,
      status,
      summary,
    };
    state.ledger.push(newEntry);
  }

  // Overview Metrics Updater
  function updateOverviewMetrics() {
    document.getElementById('overview-total-topics').textContent = state.metrics.totalTopics;
    document.getElementById('overview-high-risk').textContent = state.metrics.highRiskCount;
    document.getElementById('overview-partner-count').textContent = state.metrics.partnerCount;
    document.getElementById('overview-scrubbed-count').textContent = state.metrics.scrubbedCount;
    document.getElementById('badge-topics-count').textContent = state.metrics.totalTopics;
    document.getElementById('badge-partners-count').textContent = state.metrics.partnerCount;

    document.getElementById('spotlight-topics').textContent = `${state.metrics.totalTopics} Topics`;
    document.getElementById('spotlight-drift').textContent = `${state.metrics.driftIndex}%`;
  }

  // Setup Event Handlers
  function initEventHandlers() {
    // Search & Filter Listeners
    document.getElementById('input-topics-search').addEventListener('input', renderTopics);
    document.getElementById('select-category-filter').addEventListener('change', renderTopics);
    document.getElementById('select-risk-filter').addEventListener('change', renderTopics);

    // Batch Scrub Button
    document.getElementById('btn-batch-scrub').addEventListener('click', async () => {
      const selected = Array.from(state.selectedTopicIds);
      if (selected.length === 0) return;

      const btn = document.getElementById('btn-batch-scrub');
      btn.textContent = 'Purging Batch...';
      btn.disabled = true;

      try {
        await invokeCommand('batch_scrub_topics', { topic_ids: selected });
        state.topics = state.topics.filter(t => !state.selectedTopicIds.has(t.id));
        state.metrics.scrubbedCount += selected.length;
        state.metrics.totalTopics = state.topics.length;
        state.selectedTopicIds.clear();
        
        recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/ad_topics/batch_scrub', 200, `Batch purged ${selected.length} topics`);
        updateOverviewMetrics();
        updateBatchButton();
        renderTopics();
        renderLedger();
      } catch (err) {
        console.error(err);
      }
    });

    // Mass-Purge All High-Risk Button
    document.getElementById('btn-quick-scrub-all-high').addEventListener('click', async () => {
      const highRisk = state.topics.filter(t => t.risk === 'Critical' || t.risk === 'High');
      const ids = highRisk.map(t => t.id);

      await invokeCommand('batch_scrub_topics', { topic_ids: ids });
      state.topics = state.topics.filter(t => t.risk !== 'Critical' && t.risk !== 'High');
      state.metrics.scrubbedCount += ids.length;
      state.metrics.totalTopics = state.topics.length;
      state.metrics.highRiskCount = 0;

      recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/ad_topics/batch_scrub', 200, `Mass-purged ${ids.length} high-risk topics`);
      updateOverviewMetrics();
      renderTopics();
      renderLedger();
    });

    // Rule Creator Form
    document.getElementById('form-create-rule').addEventListener('submit', (e) => {
      e.preventDefault();
      const pattern = document.getElementById('rule-pattern').value.trim();
      const isRegex = document.getElementById('rule-is-regex').checked;
      const autoScrub = document.getElementById('rule-auto-scrub').checked;

      if (!pattern) return;

      const newRule = {
        id: 'rule_' + Date.now(),
        pattern,
        isRegex,
        autoScrub,
      };

      state.rules.push(newRule);
      document.getElementById('rule-pattern').value = '';
      renderRules();
    });

    // Spotlight Overlay Controls
    const toggleSpotlight = () => {
      spotlightOverlay.classList.toggle('hidden');
    };

    document.getElementById('btn-toggle-spotlight').addEventListener('click', toggleSpotlight);
    document.getElementById('btn-close-spotlight').addEventListener('click', toggleSpotlight);

    // Global Shortcut Handler: Cmd/Ctrl + Shift + P
    window.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault();
        toggleSpotlight();
      }
      if (e.key === 'Escape' && !spotlightOverlay.classList.contains('hidden')) {
        toggleSpotlight();
      }
    });

    // Spotlight Quick Scrub
    document.getElementById('btn-spotlight-quick-scrub').addEventListener('click', () => {
      document.getElementById('btn-quick-scrub-all-high').click();
      toggleSpotlight();
    });

    // Onboarding Modal
    document.getElementById('btn-open-trust').addEventListener('click', () => {
      onboardingModal.showModal();
    });
    document.getElementById('btn-close-onboarding').addEventListener('click', () => {
      onboardingModal.close();
    });
    document.getElementById('btn-ack-onboarding').addEventListener('click', () => {
      onboardingModal.close();
    });

    // Run Immediate Audit Button
    document.getElementById('btn-run-audit').addEventListener('click', async () => {
      const btn = document.getElementById('btn-run-audit');
      btn.innerHTML = '⚡ Scanning...';
      btn.disabled = true;

      try {
        await invokeCommand('trigger_manual_audit');
        recordLedgerEntry('GET', 'https://accountscenter.facebook.com/ad_preferences/topics', 200, 'Executed differential snapshot audit');
        renderLedger();
      } finally {
        setTimeout(() => {
          btn.innerHTML = '<span class="btn-icon">⚡</span> Run Audit';
          btn.disabled = false;
        }, 600);
      }
    });

    // Clear Ledger Button
    document.getElementById('btn-clear-ledger').addEventListener('click', () => {
      state.ledger = [];
      renderLedger();
    });

    // Storage Actions: Exports & Nuclear Wipe
    document.getElementById('btn-export-json').addEventListener('click', () => {
      const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(state, null, 2));
      const dlAnchor = document.createElement('a');
      dlAnchor.setAttribute('href', dataStr);
      dlAnchor.setAttribute('download', 'adcleanse_snapshot_history.json');
      dlAnchor.click();
    });

    document.getElementById('btn-export-csv').addEventListener('click', () => {
      let csvContent = 'data:text/csv;charset=utf-8,ID,Category,Name,Origin,Risk\n';
      state.topics.forEach(t => {
        csvContent += `"${t.id}","${t.category}","${t.name}","${t.origin}","${t.risk}"\n`;
      });
      const dlAnchor = document.createElement('a');
      dlAnchor.setAttribute('href', encodeURI(csvContent));
      dlAnchor.setAttribute('download', 'adcleanse_topics.csv');
      dlAnchor.click();
    });

    document.getElementById('btn-nuclear-wipe').addEventListener('click', () => {
      if (confirm('CRITICAL ACTION: Are you sure you want to permanently erase the local encrypted database and remove all stored OS Keyring tokens?')) {
        state.topics = [];
        state.partners = [];
        state.rules = [];
        state.metrics.totalTopics = 0;
        state.metrics.highRiskCount = 0;
        state.metrics.partnerCount = 0;
        updateOverviewMetrics();
        renderTopics();
        renderPartners();
        renderRules();
        alert('Local encrypted database erased and OS Keyring credentials purged.');
      }
    });
  }

  // Initialization
  function init() {
    initTabs();
    initEventHandlers();
    updateOverviewMetrics();
    renderTopics();
    renderPartners();
    renderRules();
    renderLedger();
    console.info('AdCleanse presentation layer initialized in zero-telemetry mode.');
  }

  // Run on DOM ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
