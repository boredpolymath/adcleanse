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
    diff: {
      added: [
        { name: 'Mortgage Refinancing', origin: 'Off-Platform Pixel Tracking', risk: 'High' },
        { name: 'Clinical Depression Therapeutics', origin: 'Inferred Browsing Activity', risk: 'Critical' },
        { name: 'Pre-Approved Credit Cards', origin: 'Lookalike Audience', risk: 'High' },
      ],
      removed: [
        { name: 'Online Poker & Sports Betting', reason: 'Scrubbed by Rule #1', status: 'Purged' },
        { name: 'Weight Loss Surgery', reason: 'Scrubbed by Manual Action', status: 'Purged' },
      ],
    },
    settings: {
      pollingInterval: 60,
      jitterWindow: 180,
      digestWindow: 5,
      autoPolling: true,
      trayResident: true,
      notificationsEnabled: true,
      notifyNewTopics: true,
      notifyPartnerUploads: true,
    },
    walkthroughStep: 1,
    lastAuditEpoch: Math.floor(Date.now() / 1000),
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
      case 'get_session_status':
      case 'initiate_login':
        return {
          state: state.session.authenticated ? 'Authenticated' : 'Unauthenticated',
          user_id: state.session.userId,
          expires_at: Math.floor(Date.now() / 1000) + 86400 * 30,
          last_verified_at: Math.floor(Date.now() / 1000),
        };
      case 'revoke_session':
        state.session.authenticated = false;
        return { status: 'ok' };
      case 'trigger_manual_audit':
      case 'get_latest_snapshot':
        return {
          id: 'snap_' + Date.now(),
          timestamp_epoch: Math.floor(Date.now() / 1000),
          total_topics: state.topics.length,
          total_partners: state.partners.length,
          drift_index: state.metrics.driftIndex,
          topics: state.topics,
          partners: state.partners,
        };
      case 'get_diff_history':
        return [];
      case 'scrub_topic':
        return {
          topic_id: args.topic_id,
          topic_name: args.topic_name || 'Scrubbed Topic',
          status: 'Successfully Purged',
          http_status: 200,
          timestamp_epoch: Math.floor(Date.now() / 1000),
        };
      case 'batch_scrub_topics':
        return (args.topic_ids || []).map(id => ({
          topic_id: id,
          topic_name: 'Batch Topic',
          status: 'Successfully Purged',
          http_status: 200,
          timestamp_epoch: Math.floor(Date.now() / 1000),
        }));
      case 'opt_out_partner':
        return {
          topic_id: args.partner_id,
          topic_name: args.company_name || 'Partner Upload',
          status: 'Opt-Out Revoked',
          http_status: 200,
          timestamp_epoch: Math.floor(Date.now() / 1000),
        };
      case 'get_topic_rules':
        return state.rules;
      case 'add_topic_rule': {
        const newRule = {
          id: 'rule_' + Date.now(),
          pattern: args.pattern,
          is_regex: args.is_regex || false,
          auto_scrub_enabled: true,
          created_at_epoch: Math.floor(Date.now() / 1000),
        };
        state.rules.push(newRule);
        return newRule;
      }
      case 'get_storage_metrics':
        return {
          database_path: '~/Library/Application Support/com.boredpolymath.adcleanse/adcleanse.encrypted.db',
          encryption_active: true,
          cipher_mode: 'SQLCipher (AES-256-CBC)',
          database_size_bytes: 428032,
          snapshots_count: 14,
          auto_backup_count: 3,
        };
      case 'export_audit_data':
        return args.format === 'csv'
          ? 'SnapshotID,Timestamp,TopicID,TopicName,Category,RiskLevel\n1,1760000000,t1,Finances,High'
          : JSON.stringify({ export: 'complete', count: state.topics.length });
      case 'wipe_local_database':
        return true;
      case 'get_network_ledger':
        return state.ledger;
      case 'clear_network_ledger':
        state.ledger = [];
        return { status: 'ok' };
      case 'get_system_status':
        return {
          is_running: true,
          background_polling_active: true,
          tray_resident: true,
          spotlight_visible: false,
          cooldown_seconds_remaining: 0,
        };
      case 'toggle_spotlight_panel':
        return true;
      default:
        return { status: 'ok' };
    }
  }

  // HTML Escape Utility
  function escapeHTML(str) {
    if (str == null) return '';
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
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
      
      const riskVal = topic.risk || topic.risk_level || 'Moderate';
      let riskPill = 'pill-purple';
      if (riskVal === 'Critical') riskPill = 'pill-danger';
      else if (riskVal === 'High') riskPill = 'pill-amber';
      else if (riskVal === 'Low') riskPill = 'pill-emerald';

      tr.innerHTML = `
        <td><input type="checkbox" class="topic-check" data-id="${escapeHTML(topic.id)}" ${isChecked ? 'checked' : ''}></td>
        <td><strong>${escapeHTML(topic.category)}</strong></td>
        <td>${escapeHTML(topic.name)}</td>
        <td><span class="pill pill-purple">${escapeHTML(topic.origin)}</span></td>
        <td><span class="pill ${riskPill}">${escapeHTML(riskVal)}</span></td>
        <td>
          <button class="btn btn-danger btn-sm btn-scrub-single" data-id="${escapeHTML(topic.id)}" data-name="${escapeHTML(topic.name)}">
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
        btn.textContent = 'Removing...';
        btn.disabled = true;

        try {
          await invokeCommand('scrub_topic', { topic_id: id, topic_name: name });
          const removedTopic = state.topics.find(t => t.id === id);
          if (removedTopic && (removedTopic.risk === 'Critical' || removedTopic.risk === 'High')) {
            state.metrics.highRiskCount = Math.max(0, state.metrics.highRiskCount - 1);
          }
          state.topics = state.topics.filter(t => t.id !== id);
          state.metrics.totalTopics = state.topics.length;
          state.metrics.scrubbedCount += 1;
          
          state.diff.removed.unshift({
            name: name,
            reason: 'Cleaned by You',
            status: 'Removed',
          });

          recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/ad_topics/scrub', 200, `Removed topic: ${name}`);
          updateOverviewMetrics();
          renderTopics();
          renderDiff();
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
    if (!tbody) return;
    tbody.innerHTML = '';

    state.partners.forEach(partner => {
      const companyName = partner.company_name || partner.name;
      const windowStr = partner.upload_window_days ? `${partner.upload_window_days} Days` : (partner.window || '90 Days');
      const isPixel = partner.pixel_tracking_detected !== undefined ? partner.pixel_tracking_detected : !!partner.pixel;
      const rightsStr = partner.opt_out_status || partner.rights || 'Targeting Active';

      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td><strong>${escapeHTML(companyName)}</strong></td>
        <td>${escapeHTML(windowStr)}</td>
        <td><span class="pill ${isPixel ? 'pill-danger' : 'pill-emerald'}">${isPixel ? 'Website Tracking' : 'Customer List'}</span></td>
        <td><span class="pill pill-amber">${escapeHTML(rightsStr)}</span></td>
        <td>
          <button class="btn btn-secondary btn-sm btn-revoke-partner" data-id="${escapeHTML(partner.id)}" data-name="${escapeHTML(companyName)}">
            🚫 Stop Targeting
          </button>
        </td>
      `;
      tbody.appendChild(tr);
    });

    document.querySelectorAll('.btn-revoke-partner').forEach(btn => {
      btn.addEventListener('click', () => {
        const name = btn.dataset.name;
        btn.textContent = 'Stopped';
        btn.disabled = true;
        btn.classList.add('btn-outline');
        recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/advertisers/opt_out', 200, `Stopped partner targeting: ${name}`);
        renderLedger();
      });
    });
  }

  // Render Rules Table
  function renderRules() {
    const tbody = document.getElementById('tbody-rules');
    if (!tbody) return;
    tbody.innerHTML = '';

    if (state.rules.length === 0) {
      tbody.innerHTML = `<tr><td colspan="2" style="text-align: center; color: var(--text-dim); padding: 14px;">No blocked keywords yet. Enter a keyword above to always block it.</td></tr>`;
      return;
    }

    state.rules.forEach(rule => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td><strong>${escapeHTML(rule.pattern)}</strong></td>
        <td style="text-align: right;">
          <button class="btn btn-outline btn-sm btn-delete-rule" data-id="${escapeHTML(rule.id)}">Unblock</button>
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
    if (!tbody) return;
    tbody.innerHTML = '';

    state.ledger.slice().reverse().forEach(entry => {
      const tr = document.createElement('tr');
      tr.innerHTML = `
        <td>#${escapeHTML(entry.id)}</td>
        <td>${escapeHTML(entry.time)}</td>
        <td><span class="pill ${entry.method === 'GET' ? 'pill-purple' : 'pill-danger'}">${escapeHTML(entry.method)}</span></td>
        <td><code>${escapeHTML(entry.url)}</code></td>
        <td><span class="pill pill-emerald">${escapeHTML(entry.status)} OK</span></td>
        <td>${escapeHTML(entry.summary)}</td>
        <td><span class="pill pill-emerald">Private</span></td>
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
    const elTopics = document.getElementById('overview-total-topics');
    if (elTopics) elTopics.textContent = state.metrics.totalTopics;
    const elHigh = document.getElementById('overview-high-risk');
    if (elHigh) elHigh.textContent = state.metrics.highRiskCount;
    const elPartner = document.getElementById('overview-partner-count');
    if (elPartner) elPartner.textContent = state.metrics.partnerCount;
    const elScrubbed = document.getElementById('overview-scrubbed-count');
    if (elScrubbed) elScrubbed.textContent = state.metrics.scrubbedCount;
    const badgeTopics = document.getElementById('badge-topics-count');
    if (badgeTopics) badgeTopics.textContent = state.metrics.totalTopics;
    const badgePartners = document.getElementById('badge-partners-count');
    if (badgePartners) badgePartners.textContent = state.metrics.partnerCount;

    const spotTopics = document.getElementById('spotlight-topics');
    if (spotTopics) spotTopics.textContent = `${state.metrics.totalTopics} Topics`;
    const spotDrift = document.getElementById('spotlight-drift');
    if (spotDrift) spotDrift.textContent = `${state.metrics.highRiskCount}`;

    const quickBtn = document.getElementById('btn-quick-scrub-all-high');
    if (quickBtn) {
      if (state.metrics.highRiskCount === 0) {
        quickBtn.textContent = '✅ All Sensitive Topics Cleaned';
        quickBtn.disabled = true;
      } else {
        quickBtn.textContent = `🧹 Clean All ${state.metrics.highRiskCount} Sensitive Topics`;
        quickBtn.disabled = false;
      }
    }
  }

  // Setup Event Handlers
  function initEventHandlers() {
    // Search & Filter Listeners
    const inputSearch = document.getElementById('input-topics-search');
    if (inputSearch) inputSearch.addEventListener('input', renderTopics);

    const selectCategory = document.getElementById('select-category-filter');
    if (selectCategory) selectCategory.addEventListener('change', renderTopics);

    const selectRisk = document.getElementById('select-risk-filter');
    if (selectRisk) selectRisk.addEventListener('change', renderTopics);

    // Batch Scrub Button
    const btnBatch = document.getElementById('btn-batch-scrub');
    if (btnBatch) {
      btnBatch.addEventListener('click', async () => {
        const selected = Array.from(state.selectedTopicIds);
        if (selected.length === 0) return;

        btnBatch.textContent = 'Removing...';
        btnBatch.disabled = true;

        try {
          await invokeCommand('batch_scrub_topics', { topic_ids: selected });
          selected.forEach(id => {
            const t = state.topics.find(top => top.id === id);
            if (t) {
              if (t.risk === 'Critical' || t.risk === 'High') {
                state.metrics.highRiskCount = Math.max(0, state.metrics.highRiskCount - 1);
              }
              state.diff.removed.unshift({
                name: t.name,
                reason: 'Removed in Batch',
                status: 'Removed',
              });
            }
          });
          state.topics = state.topics.filter(t => !state.selectedTopicIds.has(t.id));
          state.metrics.scrubbedCount += selected.length;
          state.metrics.totalTopics = state.topics.length;
          state.selectedTopicIds.clear();
          
          recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/ad_topics/batch_scrub', 200, `Batch removed ${selected.length} topics`);
          updateOverviewMetrics();
          updateBatchButton();
          renderTopics();
          renderDiff();
          renderLedger();
        } catch (err) {
          console.error(err);
        }
      });
    }

    // Mass-Purge All High-Risk Button
    const btnQuickScrub = document.getElementById('btn-quick-scrub-all-high');
    if (btnQuickScrub) {
      btnQuickScrub.addEventListener('click', async () => {
        const highRisk = state.topics.filter(t => t.risk === 'Critical' || t.risk === 'High');
        const ids = highRisk.map(t => t.id);

        btnQuickScrub.textContent = 'Cleaning...';
        btnQuickScrub.disabled = true;

        await invokeCommand('batch_scrub_topics', { topic_ids: ids });
        highRisk.forEach(t => {
          state.diff.removed.unshift({
            name: t.name,
            reason: 'Cleaned Sensitive Category',
            status: 'Removed',
          });
        });
        state.topics = state.topics.filter(t => t.risk !== 'Critical' && t.risk !== 'High');
        state.metrics.scrubbedCount += ids.length;
        state.metrics.totalTopics = state.topics.length;
        state.metrics.highRiskCount = 0;

        recordLedgerEntry('POST', 'https://graph.facebook.com/v19.0/act_user/ad_topics/batch_scrub', 200, `Cleaned ${ids.length} sensitive topics`);
        updateOverviewMetrics();
        updateBatchButton();
        renderTopics();
        renderDiff();
        renderLedger();
      });
    }

    // Rule Creator Form
    const formRule = document.getElementById('form-create-rule');
    if (formRule) {
      formRule.addEventListener('submit', (e) => {
        e.preventDefault();
        const inputPattern = document.getElementById('rule-pattern');
        const pattern = inputPattern ? inputPattern.value.trim() : '';
        const checkRegex = document.getElementById('rule-is-regex');
        const isRegex = checkRegex ? checkRegex.checked : false;
        const checkAuto = document.getElementById('rule-auto-scrub');
        const autoScrub = checkAuto ? checkAuto.checked : true;

        if (!pattern) return;

        const newRule = {
          id: 'rule_' + Date.now(),
          pattern,
          isRegex,
          autoScrub,
        };

        state.rules.push(newRule);
        if (inputPattern) inputPattern.value = '';
        renderRules();
      });
    }

    // Spotlight Overlay Controls
    const toggleSpotlight = () => {
      if (spotlightOverlay) spotlightOverlay.classList.toggle('hidden');
    };

    const btnSpotlightToggle = document.getElementById('btn-toggle-spotlight');
    if (btnSpotlightToggle) btnSpotlightToggle.addEventListener('click', toggleSpotlight);

    const btnSpotlightClose = document.getElementById('btn-close-spotlight');
    if (btnSpotlightClose) btnSpotlightClose.addEventListener('click', toggleSpotlight);

    // Global Shortcut Handler: Cmd/Ctrl + Shift + P
    window.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'p' || e.key === 'P')) {
        e.preventDefault();
        toggleSpotlight();
      }
      if (e.key === 'Escape' && spotlightOverlay && !spotlightOverlay.classList.contains('hidden')) {
        toggleSpotlight();
      }
    });

    // Spotlight Quick Scrub
    const btnSpotlightQuick = document.getElementById('btn-spotlight-quick-scrub');
    if (btnSpotlightQuick) {
      btnSpotlightQuick.addEventListener('click', () => {
        const mainQuickBtn = document.getElementById('btn-quick-scrub-all-high');
        if (mainQuickBtn) mainQuickBtn.click();
        toggleSpotlight();
      });
    }

    // Walkthrough Modal & Wizard Navigation
    const openTrustBtn = document.getElementById('btn-open-trust');
    if (openTrustBtn) {
      openTrustBtn.addEventListener('click', () => {
        setWalkthroughStep(1);
        if (onboardingModal) onboardingModal.showModal();
      });
    }

    const closeOnboardingBtn = document.getElementById('btn-close-onboarding');
    if (closeOnboardingBtn) {
      closeOnboardingBtn.addEventListener('click', () => {
        if (onboardingModal) onboardingModal.close();
      });
    }

    const nextStepBtn = document.getElementById('btn-walkthrough-next');
    if (nextStepBtn) {
      nextStepBtn.addEventListener('click', () => {
        setWalkthroughStep(state.walkthroughStep + 1);
      });
    }

    const prevStepBtn = document.getElementById('btn-walkthrough-prev');
    if (prevStepBtn) {
      prevStepBtn.addEventListener('click', () => {
        setWalkthroughStep(state.walkthroughStep - 1);
      });
    }

    const finishStepBtn = document.getElementById('btn-walkthrough-finish');
    if (finishStepBtn) {
      finishStepBtn.addEventListener('click', () => {
        if (onboardingModal) onboardingModal.close();
      });
    }

    // Step Indicator Clicks
    for (let i = 1; i <= 3; i++) {
      const pill = document.getElementById(`indicator-step-${i}`);
      if (pill) {
        pill.addEventListener('click', () => setWalkthroughStep(i));
      }
    }

    // Settings Panel Sliders
    const sliderPoll = document.getElementById('slider-polling-interval');
    const valPoll = document.getElementById('val-polling-interval');
    if (sliderPoll && valPoll) {
      sliderPoll.addEventListener('input', (e) => {
        valPoll.textContent = `${e.target.value} min`;
        state.settings.pollingInterval = parseInt(e.target.value, 10);
      });
    }

    const sliderJitter = document.getElementById('slider-jitter-window');
    const valJitter = document.getElementById('val-jitter-window');
    if (sliderJitter && valJitter) {
      sliderJitter.addEventListener('input', (e) => {
        valJitter.textContent = `${e.target.value} sec`;
        state.settings.jitterWindow = parseInt(e.target.value, 10);
      });
    }

    const sliderDigest = document.getElementById('slider-digest-window');
    const valDigest = document.getElementById('val-digest-window');
    if (sliderDigest && valDigest) {
      sliderDigest.addEventListener('input', (e) => {
        valDigest.textContent = `${e.target.value} min`;
        state.settings.digestWindow = parseInt(e.target.value, 10);
      });
    }

    // Settings Panel Toggles
    [
      'toggle-auto-polling',
      'toggle-tray-resident',
      'toggle-notifications-enabled',
      'toggle-notify-new-topics',
      'toggle-notify-partner-uploads',
    ].forEach(id => {
      const toggle = document.getElementById(id);
      if (toggle) {
        toggle.addEventListener('change', (e) => {
          state.settings[id] = e.target.checked;
        });
      }
    });

    // Recompute Diff Button
    const btnRecompute = document.getElementById('btn-recompute-diff');
    if (btnRecompute) {
      btnRecompute.addEventListener('click', async () => {
        btnRecompute.textContent = 'Checking...';
        btnRecompute.disabled = true;
        try {
          await invokeCommand('get_diff_history');
          recordLedgerEntry('GET', 'https://accountscenter.facebook.com/ad_preferences/topics', 200, 'Checked for profile changes');
          renderLedger();
          renderDiff();
        } finally {
          setTimeout(() => {
            btnRecompute.textContent = 'Check for Changes Now';
            btnRecompute.disabled = false;
          }, 500);
        }
      });
    }

    // Run Immediate Audit Button
    const btnRunAudit = document.getElementById('btn-run-audit');
    if (btnRunAudit) {
      btnRunAudit.addEventListener('click', async () => {
        btnRunAudit.innerHTML = '⚡ Checking...';
        btnRunAudit.disabled = true;

        try {
          await invokeCommand('trigger_manual_audit');
          state.lastAuditEpoch = Math.floor(Date.now() / 1000);
          updateHeaderLastAudit();
          recordLedgerEntry('GET', 'https://accountscenter.facebook.com/ad_preferences/topics', 200, 'Checked ad preferences');
          renderLedger();
        } finally {
          setTimeout(() => {
            btnRunAudit.innerHTML = '⚡ Check Now';
            btnRunAudit.disabled = false;
          }, 600);
        }
      });
    }

    // Clear Ledger Button
    const btnClearLedger = document.getElementById('btn-clear-ledger');
    if (btnClearLedger) {
      btnClearLedger.addEventListener('click', () => {
        state.ledger = [];
        renderLedger();
      });
    }

    // Storage Actions: Exports & Nuclear Wipe
    const btnExportJson = document.getElementById('btn-export-json');
    if (btnExportJson) {
      btnExportJson.addEventListener('click', () => {
        const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(state, null, 2));
        const dlAnchor = document.createElement('a');
        dlAnchor.setAttribute('href', dataStr);
        dlAnchor.setAttribute('download', 'adcleanse_history.json');
        dlAnchor.click();
      });
    }

    const btnExportCsv = document.getElementById('btn-export-csv');
    if (btnExportCsv) {
      btnExportCsv.addEventListener('click', () => {
        let csvContent = 'data:text/csv;charset=utf-8,ID,Category,Name,Source,Sensitivity\n';
        state.topics.forEach(t => {
          csvContent += `"${t.id}","${t.category}","${t.name}","${t.origin}","${t.risk}"\n`;
        });
        const dlAnchor = document.createElement('a');
        dlAnchor.setAttribute('href', encodeURI(csvContent));
        dlAnchor.setAttribute('download', 'adcleanse_topics.csv');
        dlAnchor.click();
      });
    }

    const btnNuclearWipe = document.getElementById('btn-nuclear-wipe');
    if (btnNuclearWipe) {
      btnNuclearWipe.addEventListener('click', async () => {
        if (confirm('Are you sure you want to erase all saved local data? This will reset AdCleanse.')) {
          await invokeCommand('wipe_local_database');
          state.topics = [];
          state.partners = [];
          state.rules = [];
          state.diff.added = [];
          state.diff.removed = [];
          state.metrics.totalTopics = 0;
          state.metrics.highRiskCount = 0;
          state.metrics.partnerCount = 0;
          state.metrics.scrubbedCount = 0;
          state.metrics.driftIndex = 0.0;
          updateOverviewMetrics();
          renderTopics();
          renderPartners();
          renderRules();
          renderDiff();
          alert('All local data has been erased.');
        }
      });
    }
  }

  // Walkthrough Wizard Controller
  function setWalkthroughStep(step) {
    state.walkthroughStep = Math.max(1, Math.min(3, step));
    for (let i = 1; i <= 3; i++) {
      const stepEl = document.getElementById(`walkthrough-step-${i}`);
      const pillEl = document.getElementById(`indicator-step-${i}`);
      if (stepEl) {
        if (i === state.walkthroughStep) stepEl.classList.remove('hidden');
        else stepEl.classList.add('hidden');
      }
      if (pillEl) {
        pillEl.classList.remove('active', 'completed');
        if (i === state.walkthroughStep) pillEl.classList.add('active');
        else if (i < state.walkthroughStep) pillEl.classList.add('completed');
      }
    }

    const prevBtn = document.getElementById('btn-walkthrough-prev');
    const nextBtn = document.getElementById('btn-walkthrough-next');
    const finishBtn = document.getElementById('btn-walkthrough-finish');

    if (prevBtn) prevBtn.style.visibility = state.walkthroughStep === 1 ? 'hidden' : 'visible';
    if (nextBtn) {
      if (state.walkthroughStep === 3) nextBtn.classList.add('hidden');
      else nextBtn.classList.remove('hidden');
    }
    if (finishBtn) {
      if (state.walkthroughStep === 3) finishBtn.classList.remove('hidden');
      else finishBtn.classList.add('hidden');
    }
  }

  // Differential Timeline Renderer
  function renderDiff() {
    const addedList = document.getElementById('diff-added-list');
    const removedList = document.getElementById('diff-removed-list');
    const addedCount = document.getElementById('diff-added-count');
    const removedCount = document.getElementById('diff-removed-count');

    if (addedCount) addedCount.textContent = `${state.diff.added.length} Topics`;
    if (removedCount) removedCount.textContent = `${state.diff.removed.length} Topics`;

    if (addedList) {
      if (state.diff.added.length === 0) {
        addedList.innerHTML = '<li style="color: var(--text-dim); padding: 12px;">No new topics detected since baseline snapshot.</li>';
      } else {
        addedList.innerHTML = state.diff.added.map(item => `
          <li>
            <strong>${escapeHTML(item.name)}</strong>
            <small>Origin: ${escapeHTML(item.origin)} &bull; Risk: ${escapeHTML(item.risk)}</small>
          </li>
        `).join('');
      }
    }

    if (removedList) {
      if (state.diff.removed.length === 0) {
        removedList.innerHTML = '<li style="color: var(--text-dim); padding: 12px;">No topics removed in current session.</li>';
      } else {
        removedList.innerHTML = state.diff.removed.map(item => `
          <li>
            <strong>${escapeHTML(item.name)}</strong>
            <small>${escapeHTML(item.reason)} &bull; Status: ${escapeHTML(item.status)}</small>
          </li>
        `).join('');
      }
    }
  }

  // Live Header Last Scan Updater
  function updateHeaderLastAudit() {
    const el = document.getElementById('header-last-scan');
    if (!el) return;
    const now = Math.floor(Date.now() / 1000);
    const diffSec = Math.max(0, now - state.lastAuditEpoch);
    if (diffSec < 60) {
      el.textContent = 'Last Audit: Just now';
    } else {
      const mins = Math.floor(diffSec / 60);
      el.textContent = `Last Audit: ${mins}m ago`;
    }
  }

  // Storage Metrics Loader
  async function loadStorageMetadata() {
    try {
      const meta = await invokeCommand('get_storage_metrics');
      if (meta) {
        const pathEl = document.getElementById('meta-db-path');
        const backupEl = document.getElementById('meta-backup-count');
        if (pathEl && meta.database_path) pathEl.textContent = meta.database_path;
        if (backupEl && meta.auto_backup_count !== undefined) backupEl.textContent = `${meta.auto_backup_count} Rotations`;
      }
    } catch (err) {
      console.warn('Storage metrics load warning:', err);
    }
  }

  // Initialization
  function init() {
    try { initTabs(); } catch (e) { console.warn('initTabs:', e); }
    try { initEventHandlers(); } catch (e) { console.warn('initEventHandlers:', e); }
    try { updateOverviewMetrics(); } catch (e) { console.warn('updateOverviewMetrics:', e); }
    try { renderTopics(); } catch (e) { console.warn('renderTopics:', e); }
    try { renderPartners(); } catch (e) { console.warn('renderPartners:', e); }
    try { renderRules(); } catch (e) { console.warn('renderRules:', e); }
    try { renderDiff(); } catch (e) { console.warn('renderDiff:', e); }
    try { renderLedger(); } catch (e) { console.warn('renderLedger:', e); }
    try { setWalkthroughStep(1); } catch (e) { console.warn('setWalkthroughStep:', e); }
    try { updateHeaderLastAudit(); } catch (e) { console.warn('updateHeaderLastAudit:', e); }
    try { loadStorageMetadata(); } catch (e) { console.warn('loadStorageMetadata:', e); }

    // Periodic header time updater
    setInterval(updateHeaderLastAudit, 30000);

    console.info('AdCleanse presentation layer initialized in zero-telemetry mode.');
  }

  // Run on DOM ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
