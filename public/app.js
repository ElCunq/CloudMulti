// State Management
let accountsList = [];
let zonesList = [];
let activeZone = null;

// DOM Initialization
document.addEventListener('DOMContentLoaded', () => {
  setupEventListeners();
  refreshAllData();
});

function setupEventListeners() {
  // Modal openers
  document.getElementById('btn-open-accounts').addEventListener('click', () => {
    switchAccountTab('add');
    document.getElementById('account-modal').showModal();
  });

  document.getElementById('btn-open-add-zone').addEventListener('click', () => {
    if (accountsList.length === 0) {
      showToast('Please add at least one Cloudflare Account token first!', 'error');
      document.getElementById('account-modal').showModal();
      return;
    }
    document.getElementById('add-zone-modal').showModal();
  });

  document.getElementById('btn-refresh-zones').addEventListener('click', () => {
    refreshAllData();
    showToast('Refreshing accounts and domains across edge...', 'success');
  });

  // Search & Filters
  document.getElementById('search-input').addEventListener('input', renderZonesGrid);
  document.getElementById('account-filter').addEventListener('change', renderZonesGrid);
  
  // Tunnel Search
  document.getElementById('tunnel-search-input').addEventListener('input', renderTunnelsGrid);
}

// Data Fetching
async function refreshAllData() {
  try {
    const [accountsResp, zonesResp] = await Promise.all([
      fetch('/api/accounts'),
      fetch('/api/zones')
    ]);

    if (!accountsResp.ok) throw new Error('Failed to load accounts');
    if (!zonesResp.ok) throw new Error('Failed to load zones');

    accountsList = await accountsResp.json();
    zonesList = await zonesResp.json();

    updateStats();
    populateAccountSelectors();
    renderZonesGrid();
    renderAccountsTable();

    // Async background fetches for heavier stats
    loadUnifiedAnalytics();
    loadTunnelsData();
  } catch (err) {
    console.error('Data fetch error:', err);
    showToast(`Error syncing dashboard: ${err.message}`, 'error');
  }
}

function updateStats() {
  document.getElementById('stat-accounts').textContent = accountsList.length;
  document.getElementById('stat-zones').textContent = zonesList.length;
}

function populateAccountSelectors() {
  // Filter Dropdown
  const filterSelect = document.getElementById('account-filter');
  const currentFilter = filterSelect.value;
  filterSelect.innerHTML = '<option value="all">All Accounts</option>';
  accountsList.forEach(acc => {
    const opt = document.createElement('option');
    opt.value = acc.id;
    opt.textContent = acc.account_name;
    filterSelect.appendChild(opt);
  });
  if (accountsList.some(a => a.id === currentFilter)) {
    filterSelect.value = currentFilter;
  }

  // Add Zone Account Select
  const zoneSelect = document.getElementById('select-zone-account');
  zoneSelect.innerHTML = '';
  accountsList.forEach(acc => {
    const opt = document.createElement('option');
    opt.value = acc.id;
    opt.textContent = acc.account_name;
    zoneSelect.appendChild(opt);
  });
}

// Render Zones
function renderZonesGrid() {
  const grid = document.getElementById('zones-grid');
  const searchQuery = document.getElementById('search-input').value.toLowerCase().trim();
  const accountFilter = document.getElementById('account-filter').value;

  const filtered = zonesList.filter(z => {
    const matchesSearch = z.name.toLowerCase().includes(searchQuery) || 
                          z.account_name.toLowerCase().includes(searchQuery);
    const matchesAccount = accountFilter === 'all' || z.account_id === accountFilter;
    return matchesSearch && matchesAccount;
  });

  if (filtered.length === 0) {
    grid.innerHTML = `
      <div class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
          </svg>
        </div>
        <h3>No Domains Found</h3>
        <p>No Cloudflare domains match your search query or selected account filter. Add a domain to start managing DNS and edge cache.</p>
        <button class="btn btn-primary btn-sm" onclick="document.getElementById('add-zone-modal').showModal()">+ Add New Domain</button>
      </div>
    `;
    return;
  }

  grid.innerHTML = filtered.map(z => {
    const statusClass = z.status.toLowerCase() === 'active' ? 'active' : 'pending';
    const nsText = z.name_servers && z.name_servers.length > 0 ? z.name_servers.slice(0, 2).join(', ') : 'Pending NS check';
    const isSelected = activeZone && activeZone.id === z.id;

    return `
      <div class="zone-card ${isSelected ? 'selected' : ''}" data-zone-id="${z.id}">
        <div>
          <div class="zone-card-header">
            <div class="zone-domain">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width: 20px; height: 20px; color: var(--accent-primary);">
                <circle cx="12" cy="12" r="10"/><path d="M2 12h20"/>
              </svg>
              ${z.name}
            </div>
            <span class="status-badge ${statusClass}">
              <span class="status-dot"></span>
              ${z.status}
            </span>
          </div>
          <div style="margin-top: 10px;">
            <span class="account-badge">${z.account_name}</span>
          </div>
        </div>

        <div class="zone-card-meta">
          <strong style="color: var(--text-main); font-weight: 500;">Nameservers:</strong><br>
          <span style="font-family: 'JetBrains Mono', monospace; font-size: 0.78rem;">${nsText}</span>
        </div>

        <div class="zone-quick-settings">
          <div class="quick-setting-row">
            <span style="font-weight: 500;">SSL Mode:</span>
            <select class="select-input select-xs" id="ssl-select-${z.id}" onchange="changeCardSsl('${z.id}', this)" onclick="event.stopPropagation()">
              <option value="loading" disabled selected>Loading...</option>
              <option value="off">Off</option>
              <option value="flexible">Flexible</option>
              <option value="full">Full</option>
              <option value="strict">Strict</option>
            </select>
          </div>
          <div class="quick-setting-row">
            <span style="font-weight: 500;">Under Attack:</span>
            <label class="switch" onclick="event.stopPropagation()">
              <input type="checkbox" id="waf-switch-${z.id}" onchange="changeCardUnderAttack('${z.id}', this)">
              <span class="slider round"></span>
            </label>
          </div>
          <div class="quick-setting-row">
            <span style="font-weight: 500;">Dev Mode:</span>
            <label class="switch" onclick="event.stopPropagation()">
              <input type="checkbox" id="dev-switch-${z.id}" onchange="changeCardDevMode('${z.id}', this)">
              <span class="slider round"></span>
            </label>
          </div>
        </div>

        <div class="zone-card-actions">
          <button class="btn btn-primary btn-sm" onclick='openZoneDetail(${JSON.stringify(z)})'>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            </svg>
            Manage Zone
          </button>
        </div>
      </div>
    `;
  }).join('');

  // Fetch settings asynchronously for each zone
  filtered.forEach(z => fetchZoneSettings(z.id));
}

// Account Tab Switching & CRUD
function switchAccountTab(tab) {
  const addTab = document.getElementById('tab-add-account');
  const listTab = document.getElementById('tab-list-accounts');
  const formSection = document.getElementById('form-add-account');
  const listSection = document.getElementById('section-list-accounts');

  if (tab === 'add') {
    addTab.classList.add('active');
    listTab.classList.remove('active');
    formSection.style.display = 'block';
    listSection.style.display = 'none';
  } else {
    listTab.classList.add('active');
    addTab.classList.remove('active');
    formSection.style.display = 'none';
    listSection.style.display = 'block';
    renderAccountsTable();
  }
}

function renderAccountsTable() {
  const container = document.getElementById('accounts-table-container');
  if (accountsList.length === 0) {
    container.innerHTML = `<div style="padding: 32px; text-align: center; color: var(--text-muted);">No accounts stored yet. Add one in the tab above.</div>`;
    return;
  }

  container.innerHTML = `
    <table class="data-table">
      <thead>
        <tr>
          <th>Account Label</th>
          <th>Added On</th>
          <th>Token Status</th>
          <th style="text-align: right;">Action</th>
        </tr>
      </thead>
      <tbody>
        ${accountsList.map(acc => `
          <tr>
            <td><strong style="color: var(--text-main);">${acc.account_name}</strong></td>
            <td>${new Date(acc.created_at).toLocaleDateString()}</td>
            <td>
              <span class="status-badge active" style="font-size: 0.7rem;">
                <span class="status-dot"></span> AES-GCM Encrypted
              </span>
            </td>
            <td style="text-align: right;">
              <button class="btn btn-danger btn-sm" onclick="handleDeleteAccount('${acc.id}', '${acc.account_name}')">Delete</button>
            </td>
          </tr>
        `).join('')}
      </tbody>
    </table>
  `;
}

async function handleCreateAccount(event) {
  event.preventDefault();
  const btn = document.getElementById('btn-submit-account');
  const nameInput = document.getElementById('input-account-name');
  const tokenInput = document.getElementById('input-api-token');

  btn.disabled = true;
  btn.textContent = 'Verifying with Cloudflare...';

  try {
    const resp = await fetch('/api/accounts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        account_name: nameInput.value.trim(),
        api_token: tokenInput.value.trim()
      })
    });

    const data = await resp.json();
    if (!resp.ok) {
      throw new Error(data.error || 'Token verification failed');
    }

    showToast(`Account "${data.account_name}" verified & encrypted successfully!`, 'success');
    nameInput.value = '';
    tokenInput.value = '';
    document.getElementById('account-modal').close();
    await refreshAllData();
  } catch (err) {
    showToast(`Failed to add account: ${err.message}`, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save & Verify Token';
  }
}

async function handleDeleteAccount(id, name) {
  if (!confirm(`Are you sure you want to delete account "${name}"? All associated zone mappings in cache will be removed.`)) return;

  try {
    const resp = await fetch(`/api/accounts/${id}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error('Delete failed on server');

    showToast(`Account "${name}" deleted.`, 'success');
    await refreshAllData();
  } catch (err) {
    showToast(`Error deleting account: ${err.message}`, 'error');
  }
}

// Zone Creation
async function handleCreateZone(event) {
  event.preventDefault();
  const btn = document.getElementById('btn-submit-zone');
  const domainInput = document.getElementById('input-domain-name');
  const accountSelect = document.getElementById('select-zone-account');

  btn.disabled = true;
  btn.textContent = 'Creating on Cloudflare...';

  try {
    const resp = await fetch('/api/zones', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        domain_name: domainInput.value.trim(),
        account_id: accountSelect.value
      })
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || 'Failed to create zone');

    showToast(`Domain "${data.name}" added to Cloudflare!`, 'success');
    domainInput.value = '';
    document.getElementById('add-zone-modal').close();
    await refreshAllData();
  } catch (err) {
    showToast(`Error adding domain: ${err.message}`, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Create Domain';
  }
}

// Zone Detail Modal & Tabs
function switchZoneDetailTab(tab) {
  const dnsTabBtn = document.getElementById('tab-zone-dns');
  const cacheTabBtn = document.getElementById('tab-zone-cache');
  const dnsSection = document.getElementById('section-zone-dns');
  const cacheSection = document.getElementById('section-zone-cache');

  if (tab === 'dns') {
    dnsTabBtn.classList.add('active');
    cacheTabBtn.classList.remove('active');
    dnsSection.style.display = 'block';
    cacheSection.style.display = 'none';
  } else {
    cacheTabBtn.classList.add('active');
    dnsTabBtn.classList.remove('active');
    dnsSection.style.display = 'none';
    cacheSection.style.display = 'block';
  }
}

async function openZoneDetail(zone) {
  activeZone = zone;
  document.getElementById('detail-zone-title').textContent = zone.name;
  document.getElementById('detail-zone-badge').textContent = zone.account_name;
  switchZoneDetailTab('dns');

  // Highlight selected card
  document.querySelectorAll('.zone-card').forEach(card => {
    if (card.dataset.zoneId === zone.id) {
      card.classList.add('selected');
    } else {
      card.classList.remove('selected');
    }
  });

  // Reveal inline zone panel right under domains grid
  const panel = document.getElementById('inline-zone-panel');
  panel.style.display = 'block';
  panel.scrollIntoView({ behavior: 'smooth', block: 'start' });

  await loadDnsRecords();
  await loadSslSetting();
}

function closeInlineZonePanel() {
  const panel = document.getElementById('inline-zone-panel');
  if (panel) panel.style.display = 'none';
  document.querySelectorAll('.zone-card').forEach(card => card.classList.remove('selected'));
  activeZone = null;
}

function escapeHtml(str) {
  if (!str) return '';
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

// DNS CRUD
async function loadDnsRecords() {
  const container = document.getElementById('dns-table-container');
  container.innerHTML = `<div style="padding: 32px; text-align: center; color: var(--text-muted);">Loading DNS records from edge...</div>`;

  try {
    const resp = await fetch(`/api/zones/${activeZone.id}/dns`);
    if (!resp.ok) {
      const err = await resp.json();
      throw new Error(err.error || 'Failed to fetch DNS');
    }

    const records = await resp.json();
    if (records.length === 0) {
      container.innerHTML = `<div style="padding: 32px; text-align: center; color: var(--text-muted);">No DNS records found for this domain. Add one above.</div>`;
      return;
    }

    container.innerHTML = `
      <table class="data-table">
        <thead>
          <tr>
            <th>Type</th>
            <th>Name</th>
            <th>Content</th>
            <th>TTL</th>
            <th>Proxy Status</th>
            <th style="text-align: right;">Actions</th>
          </tr>
        </thead>
        <tbody>
          ${records.map(r => {
            const hasProxy = r.type === 'A' || r.type === 'CNAME';
            const isMx = r.type === 'MX';
            const displayProxy = hasProxy 
              ? `<span class="proxy-toggle-badge ${r.proxied ? 'proxied' : 'dns-only'}">${r.proxied ? '☁️ Proxied' : '⚪ DNS Only'}</span>`
              : `<span style="color: var(--text-dim); font-size: 0.8rem;">—</span>`;

            const priorityBadge = isMx && r.priority !== undefined ? ` <span class="code-pill" style="background: hsla(210, 100%, 56%, 0.15); color: var(--accent-secondary); font-size: 0.7rem; font-weight: normal; margin-left: 6px;">Priority: ${r.priority}</span>` : '';

            return `
              <tr>
                <td><span class="code-pill">${r.type}</span></td>
                <td><strong style="color: var(--text-main);">${escapeHtml(r.name)}</strong>${priorityBadge}</td>
                <td><span style="font-family: 'JetBrains Mono', monospace; word-break: break-all;">${escapeHtml(r.content)}</span></td>
                <td>${r.ttl === 1 ? 'Auto' : `${r.ttl}s`}</td>
                <td>${displayProxy}</td>
                <td style="text-align: right; white-space: nowrap;">
                  <button class="btn btn-secondary btn-sm edit-dns-btn"
                          data-id="${r.id}"
                          data-type="${r.type}"
                          data-name="${escapeHtml(r.name)}"
                          data-content="${escapeHtml(r.content)}"
                          data-ttl="${r.ttl}"
                          data-proxied="${r.proxied}"
                          data-priority="${r.priority || ''}"
                          style="margin-right: 6px;">
                    Edit
                  </button>
                  <button class="btn btn-danger btn-sm delete-dns-btn"
                          data-id="${r.id}"
                          data-name="${escapeHtml(r.name)}">
                    Sil
                  </button>
                </td>
              </tr>
            `;
          }).join('')}
        </tbody>
      </table>
    `;

    // Bind Edit Event Listeners
    container.querySelectorAll('.edit-dns-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const id = btn.dataset.id;
        const type = btn.dataset.type;
        const name = btn.dataset.name;
        const content = btn.dataset.content;
        const ttl = btn.dataset.ttl;
        const proxied = btn.dataset.proxied === 'true';
        const priority = btn.dataset.priority;
        openEditDnsModal(id, type, name, content, ttl, proxied, priority);
      });
    });

    // Bind Delete Event Listeners
    container.querySelectorAll('.delete-dns-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        handleDeleteDnsRecord(btn.dataset.id, btn.dataset.name);
      });
    });

  } catch (err) {
    container.innerHTML = `<div style="padding: 24px; color: var(--status-error);">Error loading DNS: ${err.message}</div>`;
  }
}

// Add Form Type change listener
function onAddTypeChange() {
  const type = document.getElementById('dns-type').value;
  const contentInput = document.getElementById('dns-content');
  const proxyContainer = document.getElementById('dns-proxy-container');
  const priorityContainer = document.getElementById('dns-priority-container');

  proxyContainer.style.display = 'none';
  priorityContainer.style.display = 'none';

  if (type === 'A') {
    contentInput.placeholder = 'IPv4 address (e.g. 1.2.3.4)';
    proxyContainer.style.display = 'flex';
  } else if (type === 'CNAME') {
    contentInput.placeholder = 'Target domain (e.g. target.com)';
    proxyContainer.style.display = 'flex';
  } else if (type === 'TXT') {
    contentInput.placeholder = 'TXT content (e.g. v=spf1...)';
  } else if (type === 'MX') {
    contentInput.placeholder = 'Mail server (e.g. mail.domain.com)';
    priorityContainer.style.display = 'flex';
  } else if (type === 'NS') {
    contentInput.placeholder = 'Nameserver (e.g. ns1.domain.com)';
  }
}

// Edit Form Type change listener
function onEditTypeChange() {
  const type = document.getElementById('edit-dns-type').value;
  const contentLabel = document.getElementById('edit-dns-content-label');
  const proxyContainer = document.getElementById('edit-dns-proxy-container');
  const priorityContainer = document.getElementById('edit-dns-priority-container');

  proxyContainer.style.display = 'none';
  priorityContainer.style.display = 'none';

  if (type === 'A') {
    contentLabel.textContent = 'IPv4 address (e.g. 1.2.3.4)';
    proxyContainer.style.display = 'flex';
  } else if (type === 'CNAME') {
    contentLabel.textContent = 'Target domain (e.g. target.com)';
    proxyContainer.style.display = 'flex';
  } else if (type === 'TXT') {
    contentLabel.textContent = 'TXT content (e.g. v=spf1...)';
  } else if (type === 'MX') {
    contentLabel.textContent = 'Mail server (e.g. mail.domain.com)';
    priorityContainer.style.display = 'block';
  } else if (type === 'NS') {
    contentLabel.textContent = 'Nameserver (e.g. ns1.domain.com)';
  }
}

function openEditDnsModal(recordId, type, name, content, ttl, proxied, priority) {
  document.getElementById('edit-dns-id').value = recordId;
  document.getElementById('edit-dns-type').value = type;
  document.getElementById('edit-dns-name').value = name;
  document.getElementById('edit-dns-content').value = content;
  document.getElementById('edit-dns-ttl').value = ttl;
  document.getElementById('edit-dns-proxied').checked = proxied;
  document.getElementById('edit-dns-priority').value = priority || '10';

  // Toggle visible form fields
  onEditTypeChange();

  document.getElementById('edit-dns-modal').showModal();
}

async function handleUpdateDnsRecord(event) {
  event.preventDefault();
  const btn = document.getElementById('btn-submit-edit-dns');
  const recordId = document.getElementById('edit-dns-id').value;
  const typeVal = document.getElementById('edit-dns-type').value;
  const nameVal = document.getElementById('edit-dns-name').value.trim();
  const contentVal = document.getElementById('edit-dns-content').value.trim();
  const ttlVal = parseInt(document.getElementById('edit-dns-ttl').value || '1', 10);
  const proxiedVal = document.getElementById('edit-dns-proxied').checked;
  const priorityVal = typeVal === 'MX' ? parseInt(document.getElementById('edit-dns-priority').value || '10', 10) : null;

  btn.disabled = true;
  btn.textContent = 'Saving...';

  const payload = {
    type: typeVal,
    name: nameVal,
    content: contentVal,
    ttl: ttlVal,
    proxied: (typeVal === 'A' || typeVal === 'CNAME') ? proxiedVal : false
  };

  if (priorityVal !== null) {
    payload.priority = priorityVal;
  }

  try {
    const resp = await fetch(`/api/zones/${activeZone.id}/dns/${recordId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || 'Failed to update DNS record');

    showToast(`DNS record "${nameVal}" updated successfully!`, 'success');
    document.getElementById('edit-dns-modal').close();
    await loadDnsRecords();
  } catch (err) {
    showToast(`Error updating DNS record: ${err.message}`, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Save Changes';
  }
}

async function handleCreateDnsRecord(event) {
  event.preventDefault();
  const btn = document.getElementById('btn-submit-dns');
  const typeVal = document.getElementById('dns-type').value;
  const nameVal = document.getElementById('dns-name').value.trim();
  const contentVal = document.getElementById('dns-content').value.trim();
  const ttlVal = parseInt(document.getElementById('dns-ttl').value || '1', 10);
  const proxiedVal = document.getElementById('dns-proxied').checked;
  const priorityVal = typeVal === 'MX' ? parseInt(document.getElementById('dns-priority').value || '10', 10) : null;

  btn.disabled = true;
  btn.textContent = 'Adding...';

  const payload = {
    type: typeVal,
    name: nameVal,
    content: contentVal,
    ttl: ttlVal,
    proxied: (typeVal === 'A' || typeVal === 'CNAME') ? proxiedVal : false
  };

  if (priorityVal !== null) {
    payload.priority = priorityVal;
  }

  try {
    const resp = await fetch(`/api/zones/${activeZone.id}/dns`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || 'Failed to add DNS record');

    showToast(`DNS ${typeVal} record added!`, 'success');
    document.getElementById('dns-name').value = '';
    document.getElementById('dns-content').value = '';
    await loadDnsRecords();
  } catch (err) {
    showToast(`Error adding DNS record: ${err.message}`, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Add';
  }
}

async function handleDeleteDnsRecord(recordId, recordName) {
  if (!confirm(`Delete DNS record "${recordName}"?`)) return;

  try {
    const resp = await fetch(`/api/zones/${activeZone.id}/dns/${recordId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error('Delete failed');

    showToast(`Record "${recordName}" deleted.`, 'success');
    await loadDnsRecords();
  } catch (err) {
    showToast(`Error deleting record: ${err.message}`, 'error');
  }
}

// Quick Operations (Purge Cache & SSL)
async function handlePurgeCache() {
  const btn = document.getElementById('btn-purge-cache');
  if (!confirm(`Purge ALL cached edge files for ${activeZone.name}?`)) return;

  btn.disabled = true;
  btn.textContent = 'Purging Edge Cache...';

  try {
    const resp = await fetch(`/api/zones/${activeZone.id}/purge-cache`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ purge_everything: true })
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || 'Failed to purge cache');

    showToast(`⚡ All edge cache for ${activeZone.name} purged instantly!`, 'success');
  } catch (err) {
    showToast(`Error purging cache: ${err.message}`, 'error');
  } finally {
    btn.disabled = false;
    btn.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width:18px;height:18px;"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg> Purge Everything from Cache`;
  }
}

async function loadSslSetting() {
  // Highlight currently active option if known or reset
  document.querySelectorAll('.ssl-option').forEach(o => o.classList.remove('active'));
}

async function handleUpdateSsl(mode) {
  try {
    const resp = await fetch(`/api/zones/${activeZone.id}/ssl`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ value: mode })
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || 'Failed to update SSL');

    document.querySelectorAll('.ssl-option').forEach(o => o.classList.remove('active'));
    const clickedOpt = document.getElementById(`ssl-${mode}`);
    if (clickedOpt) clickedOpt.classList.add('active');

    showToast(`🔒 SSL/TLS encryption mode set to "${mode.toUpperCase()}"`, 'success');
  } catch (err) {
    showToast(`Error setting SSL mode: ${err.message}`, 'error');
  }
}

// Toast System
function showToast(message, type = 'success') {
  const container = document.getElementById('toast-container');
  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;
  
  const icon = type === 'success' 
    ? `<svg viewBox="0 0 24 24" fill="none" stroke="var(--status-active)" stroke-width="2.5" style="width:20px;height:20px;"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>`
    : `<svg viewBox="0 0 24 24" fill="none" stroke="var(--status-error)" stroke-width="2.5" style="width:20px;height:20px;"><circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/></svg>`;

  toast.innerHTML = `${icon} <span style="font-size: 0.9rem; font-weight: 500;">${message}</span>`;
  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transform = 'translateY(10px) scale(0.95)';
    toast.style.transition = 'all 0.25s ease';
    setTimeout(() => toast.remove(), 250);
  }, 4500);
}

// On-demand settings loading for each card
async function fetchZoneSettings(zoneId) {
  try {
    const resp = await fetch(`/api/zones/${zoneId}/settings`);
    if (!resp.ok) throw new Error('Settings fetch failed');
    const settings = await resp.json();

    const sslSelect = document.getElementById(`ssl-select-${zoneId}`);
    const wafSwitch = document.getElementById(`waf-switch-${zoneId}`);
    const devSwitch = document.getElementById(`dev-switch-${zoneId}`);

    if (sslSelect) sslSelect.value = settings.ssl_mode;
    if (wafSwitch) wafSwitch.checked = (settings.security_level === 'under_attack');
    if (devSwitch) devSwitch.checked = (settings.development_mode === 'on');
  } catch (err) {
    console.error(`Error loading settings for zone ${zoneId}:`, err);
    const sslSelect = document.getElementById(`ssl-select-${zoneId}`);
    if (sslSelect) {
      sslSelect.innerHTML = '<option value="error" disabled selected>Offline</option>';
    }
  }
}

async function changeCardSsl(zoneId, selectElem) {
  const originalVal = selectElem.value;
  selectElem.disabled = true;
  try {
    const resp = await fetch(`/api/zones/${zoneId}/settings`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssl_mode: selectElem.value })
    });
    if (!resp.ok) throw new Error('Failed to update SSL mode');
    showToast('SSL/TLS Encryption Mode updated!', 'success');
  } catch (err) {
    showToast(`Error: ${err.message}`, 'error');
    selectElem.value = originalVal;
  } finally {
    selectElem.disabled = false;
  }
}

async function changeCardUnderAttack(zoneId, checkboxElem) {
  const originalState = checkboxElem.checked;
  checkboxElem.disabled = true;
  const securityLevel = originalState ? 'under_attack' : 'medium';
  try {
    const resp = await fetch(`/api/zones/${zoneId}/settings`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ security_level: securityLevel })
    });
    if (!resp.ok) throw new Error('Failed to update Security Level');
    showToast(originalState ? 'WAF Under Attack Mode ACTIVATED! 🛡️' : 'WAF Under Attack Mode deactivated.', 'success');
  } catch (err) {
    showToast(`Error: ${err.message}`, 'error');
    checkboxElem.checked = !originalState;
  } finally {
    checkboxElem.disabled = false;
  }
}

async function changeCardDevMode(zoneId, checkboxElem) {
  const originalState = checkboxElem.checked;
  checkboxElem.disabled = true;
  const devMode = originalState ? 'on' : 'off';
  try {
    const resp = await fetch(`/api/zones/${zoneId}/settings`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ development_mode: devMode })
    });
    if (!resp.ok) throw new Error('Failed to update Development Mode');
    showToast(originalState ? 'Development Mode enabled (bypass cache for 3h)' : 'Development Mode disabled.', 'success');
  } catch (err) {
    showToast(`Error: ${err.message}`, 'error');
    checkboxElem.checked = !originalState;
  } finally {
    checkboxElem.disabled = false;
  }
}

// Unified GraphQL Analytics
async function loadUnifiedAnalytics() {
  const reqCard = document.getElementById('stat-requests');
  const bwCard = document.getElementById('stat-bandwidth');
  const visCard = document.getElementById('stat-visitors');

  reqCard.textContent = '...';
  bwCard.textContent = '...';
  visCard.textContent = '...';

  try {
    const resp = await fetch('/api/analytics');
    if (!resp.ok) throw new Error('Failed to load analytics');
    const data = await resp.json();

    reqCard.textContent = data.total_requests.toLocaleString();
    visCard.textContent = data.unique_visitors.toLocaleString();
    
    // Format bandwidth dynamically
    const bytes = data.total_bandwidth_bytes;
    if (bytes >= 1e9) {
      bwCard.textContent = `${(bytes / 1e9).toFixed(2)} GB`;
    } else {
      bwCard.textContent = `${(bytes / 1e6).toFixed(2)} MB`;
    }
  } catch (err) {
    console.error('Analytics load error:', err);
    reqCard.textContent = 'N/A';
    bwCard.textContent = 'N/A';
    visCard.textContent = 'N/A';
  }
}

// Cloudflare Tunnels (Zero Trust)
let tunnelsList = [];

async function loadTunnelsData() {
  const tunnelStat = document.getElementById('stat-tunnels');
  tunnelStat.textContent = 'Loading...';

  try {
    const resp = await fetch('/api/tunnels');
    if (!resp.ok) throw new Error('Failed to load tunnels');
    tunnelsList = await resp.json();

    const healthyCount = tunnelsList.filter(t => t.status === 'healthy').length;
    tunnelStat.textContent = `${healthyCount} / ${tunnelsList.length}`;

    renderTunnelsGrid();
  } catch (err) {
    console.error('Tunnels load error:', err);
    tunnelStat.textContent = 'Error';
    const tbody = document.getElementById('tunnels-table-body');
    if (tbody) {
      tbody.innerHTML = `<tr><td colspan="5" style="text-align: center; color: var(--status-error); padding: 24px;">Failed to load tunnels: ${err.message}</td></tr>`;
    }
  }
}

function renderTunnelsGrid() {
  const tbody = document.getElementById('tunnels-table-body');
  if (!tbody) return;

  const searchQuery = document.getElementById('tunnel-search-input').value.toLowerCase().trim();
  const filtered = tunnelsList.filter(t => 
    t.name.toLowerCase().includes(searchQuery) ||
    t.account_name.toLowerCase().includes(searchQuery)
  );

  if (filtered.length === 0) {
    tbody.innerHTML = `<tr><td colspan="5" style="text-align: center; color: var(--text-muted); padding: 24px;">No tunnels found.</td></tr>`;
    return;
  }

  tbody.innerHTML = filtered.map(t => {
    let statusClass = 'status-tunnel-inactive';
    if (t.status === 'healthy') statusClass = 'status-tunnel-healthy';
    else if (t.status === 'down') statusClass = 'status-tunnel-down';
    else if (t.status === 'degraded') statusClass = 'status-tunnel-degraded';

    return `
      <tr>
        <td><strong style="color: var(--text-main);">${t.name}</strong></td>
        <td><span class="${statusClass}">${t.status}</span></td>
        <td><span style="font-family: 'JetBrains Mono', monospace; font-size: 0.8rem; word-break: break-all;">${t.account_id}</span></td>
        <td><span class="account-badge">${t.account_name}</span></td>
        <td><span style="font-family: 'JetBrains Mono', monospace; font-size: 0.8rem; word-break: break-all;">${t.tunnel_id}</span></td>
      </tr>
    `;
  }).join('');
}

// Main Tab Switcher
function switchMainView(view) {
  const zonesView = document.getElementById('zones-view-section');
  const tunnelsView = document.getElementById('tunnels-view-section');
  const zonesTab = document.getElementById('view-tab-zones');
  const tunnelsTab = document.getElementById('view-tab-tunnels');

  if (view === 'zones') {
    zonesView.style.display = 'block';
    tunnelsView.style.display = 'none';
    zonesTab.classList.add('active');
    tunnelsTab.classList.remove('active');
  } else {
    zonesView.style.display = 'none';
    tunnelsView.style.display = 'block';
    zonesTab.classList.remove('active');
    tunnelsTab.classList.add('active');
    loadTunnelsData();
  }
}
