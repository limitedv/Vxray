const { invoke } = window.__TAURI__.core;

// ===== State =====
let selectedServerLink = null;
let connectedServerLink = null;
let isRunning = false;
let isConnecting = false;
let isInvoking = false;
let testingActive = false;
let logsOpen = false;
let livePingInterval = null;
let crashCheckInterval = null;
let connectStartTime = 0;
let consecutivePingFailures = 0;
let originalIp = null;
let isTunRouted = false;

// ===== DOM Refs =====
const serverList = document.getElementById('server-list');
const logViewer = document.getElementById('log-viewer');
const btnConnect = document.getElementById('btn-connect');
const btnTest = document.getElementById('btn-test');
const btnLogs = document.getElementById('btn-logs');
const statusLabel = document.getElementById('status-label');
const livePing = document.getElementById('live-ping');
const flagImg = document.getElementById('flag-img');
const countryName = document.getElementById('country-name');
const ipAddress = document.getElementById('ip-address');
const tunToggle = document.getElementById('tun-toggle');
const btnUpdateSubs = document.getElementById('btn-update-subs');
const toggleAutoReconnect = document.getElementById('toggle-auto-reconnect');
const toggleAutoStart = document.getElementById('toggle-auto-start');
const toggleStartMinimized = document.getElementById('toggle-start-minimized');
const rowStartMinimized = document.getElementById('row-start-minimized');

// ===== Initialization =====
async function initApp() {
  // Titlebar controls
  const minBtn = document.getElementById('titlebar-minimize');
  if (minBtn) minBtn.addEventListener('click', () => invoke('minimize_window'));

  const maxBtn = document.getElementById('titlebar-maximize');
  if (maxBtn) maxBtn.addEventListener('click', () => invoke('maximize_window'));

  const closeBtn = document.getElementById('titlebar-close');
  if (closeBtn) closeBtn.addEventListener('click', () => invoke('close_window'));

  // Load TUN state
  try {
    const tunEnabled = await invoke('get_tun_enabled');
    if (tunToggle) tunToggle.checked = tunEnabled;
  } catch (_) {}

  // Load Startup Settings
  try {
    const startup = await invoke('get_startup_settings');
    if (toggleAutoReconnect) toggleAutoReconnect.checked = startup.auto_reconnect;
    if (toggleAutoStart) toggleAutoStart.checked = startup.auto_start;
    if (toggleStartMinimized) toggleStartMinimized.checked = startup.start_minimized;
    if (startup.auto_start && rowStartMinimized) {
      rowStartMinimized.classList.add('enabled');
    }
  } catch (_) {}

  // Auto Reconnect Logic
  try {
    const info = await invoke('get_auto_reconnect_info');
    if (info.auto_reconnect && info.link && info.link !== "") {
      if (tunToggle) tunToggle.checked = info.tun_mode;
      setTimeout(() => {
        selectedServerLink = info.link;
        connectToLink(info.link, false);
      }, 500);
    }
  } catch (_) {}

  // Toggles handlers
  if (tunToggle) tunToggle.addEventListener('change', handleTunToggle);
  
  if (toggleAutoReconnect) {
    toggleAutoReconnect.addEventListener('change', async (e) => {
      await invoke('set_auto_reconnect', { enabled: e.target.checked });
    });
  }

  if (toggleAutoStart) {
    toggleAutoStart.addEventListener('change', async (e) => {
      const enabled = e.target.checked;
      await invoke('set_auto_start', { enabled });
      if (rowStartMinimized) {
        if (enabled) {
          rowStartMinimized.classList.add('enabled');
        } else {
          rowStartMinimized.classList.remove('enabled');
        }
      }
    });
  }

  if (toggleStartMinimized) {
    toggleStartMinimized.addEventListener('change', async (e) => {
      await invoke('set_start_minimized', { enabled: e.target.checked });
    });
  }

  // Load servers
  setTimeout(refreshServerList, 50);
  setTimeout(() => checkIp(false), 300);
  setTimeout(runLivePing, 600);
  setTimeout(updateAllSubscriptions, 2000);

  // Start timers
  livePingInterval = setInterval(runLivePing, 4000);
  crashCheckInterval = setInterval(checkCoreHealth, 2000);

  // Global drag handling to absolutely prevent the "not allowed" cross cursor anywhere in the window
  document.addEventListener('dragenter', (e) => {
    if (isDraggingGroup) {
      e.preventDefault();
    }
  });
  
  document.addEventListener('dragover', (e) => {
    if (isDraggingGroup) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
    }
  });

  document.addEventListener('drop', (e) => {
    if (isDraggingGroup) {
      e.preventDefault();
    }
  });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}

// ===== Sidebar Toggle =====
function toggleSidebar() {
  document.getElementById('sidebar').classList.toggle('collapsed');
}

// ===== Tab Switching =====
function switchTab(index) {
  document.getElementById('tab-servers').classList.toggle('active', index === 0);
  document.getElementById('tab-settings').classList.toggle('active', index === 1);
  document.getElementById('page-servers').classList.toggle('active', index === 0);
  document.getElementById('page-settings').classList.toggle('active', index === 1);
}

// ===== Server List =====
let collapsedGroups = {};

let draggedGroupKey = null;
let isDraggingGroup = false;

async function togglePinSubscription(groupKey) {
  try {
    await invoke('toggle_pin_subscription', { groupKey });
    await refreshServerList();
  } catch (e) {
    log('Failed to pin subscription: ' + e);
  }
}

async function refreshServerList() {
  try {
    const rawData = await invoke('get_servers');
    serverList.innerHTML = '';
    const parsed = typeof rawData === 'string' ? JSON.parse(rawData) : rawData;

    let groupsMap = {};
    let pinnedList = [];
    let orderList = [];

    if (parsed.groups) {
      groupsMap = parsed.groups;
      pinnedList = parsed.pinned || [];
      orderList = parsed.order || [];
    } else {
      groupsMap = parsed;
    }

    const groupEntries = Object.entries(groupsMap);

    // Sort entries: manual first, then pinned, then ordered/default
    groupEntries.sort(([keyA], [keyB]) => {
      if (keyA === 'manual') return -1;
      if (keyB === 'manual') return 1;

      const isPinnedA = pinnedList.includes(keyA);
      const isPinnedB = pinnedList.includes(keyB);

      if (isPinnedA && !isPinnedB) return -1;
      if (!isPinnedA && isPinnedB) return 1;

      const idxA = orderList.indexOf(keyA);
      const idxB = orderList.indexOf(keyB);

      if (idxA !== -1 && idxB !== -1) return idxA - idxB;
      if (idxA !== -1) return -1;
      if (idxB !== -1) return 1;
      return 0;
    });

    for (const [groupKey, groupData] of groupEntries) {
      const servers = groupData.servers || [];
      const isCollapsed = collapsedGroups[groupKey] || false;
      const isPinned = pinnedList.includes(groupKey);

      // Group container
      const groupEl = document.createElement('div');
      groupEl.className = 'sub-group' + (isCollapsed ? ' collapsed' : '') + (isPinned ? ' is-pinned' : '');
      groupEl.dataset.groupKey = groupKey;

      // Group header
      const header = document.createElement('div');
      header.className = 'group-header';
      header.draggable = true;

      // Drag and drop event listeners
      header.addEventListener('dragstart', (e) => {
        if (e.target.closest('.group-dropdown-menu') || e.target.closest('.group-action-btn')) {
          e.preventDefault();
          return;
        }
        isDraggingGroup = true;
        draggedGroupKey = groupKey;
        // Defer adding the class so the browser captures the original look for the drag image
        setTimeout(() => {
          groupEl.classList.add('dragging');
        }, 0);
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', groupKey);
      });

      header.addEventListener('dragend', () => {
        groupEl.classList.remove('dragging');
        document.querySelectorAll('.sub-group').forEach(el => {
          el.classList.remove('drag-target-above', 'drag-target-below');
        });
        setTimeout(() => {
          isDraggingGroup = false;
          draggedGroupKey = null;
        }, 100);
      });

      // Handle drop targets on groupEl to allow dropping anywhere in the group
      groupEl.addEventListener('dragenter', (e) => {
        if (!draggedGroupKey) return;
        e.preventDefault(); // Necessary for allowing drop
      });

      groupEl.addEventListener('dragover', (e) => {
        if (!draggedGroupKey) return;
        e.preventDefault(); // ALWAYS prevent default so the cross cursor disappears
        e.dataTransfer.dropEffect = 'move';

        if (draggedGroupKey === groupKey) return; // Don't draw blue lines over itself

        const rect = groupEl.getBoundingClientRect();
        const midpoint = rect.top + rect.height / 2;

        // Clear all previous targets
        document.querySelectorAll('.sub-group').forEach(el => {
          el.classList.remove('drag-target-above', 'drag-target-below');
        });

        if (e.clientY < midpoint) {
          groupEl.classList.add('drag-target-above');
        } else {
          groupEl.classList.add('drag-target-below');
        }
      });

      groupEl.addEventListener('dragleave', (e) => {
        // Prevent flickering by ensuring we actually left the group element
        if (!groupEl.contains(e.relatedTarget)) {
          groupEl.classList.remove('drag-target-above', 'drag-target-below');
        }
      });

      groupEl.addEventListener('drop', async (e) => {
        e.preventDefault();
        groupEl.classList.remove('drag-target-above', 'drag-target-below');

        if (!draggedGroupKey || draggedGroupKey === groupKey) return;

        const rect = groupEl.getBoundingClientRect();
        const midpoint = rect.top + rect.height / 2;
        const placeBefore = e.clientY < midpoint;

        // Find the dragged element securely without querySelector
        const draggedEl = Array.from(serverList.querySelectorAll('.sub-group'))
          .find(el => el.dataset.groupKey === draggedGroupKey);
          
        if (draggedEl && groupEl !== draggedEl) {
          if (placeBefore) {
            serverList.insertBefore(draggedEl, groupEl);
          } else {
            serverList.insertBefore(draggedEl, groupEl.nextSibling);
          }

          const newOrder = Array.from(serverList.querySelectorAll('.sub-group'))
            .map(el => el.dataset.groupKey);

          try {
            await invoke('save_subscription_order', { order: newOrder });
          } catch (_) {}
        }
      });

      // Chevron
      const chevron = document.createElement('span');
      chevron.className = 'group-chevron';
      chevron.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>';

      // Name
      const nameEl = document.createElement('span');
      nameEl.className = 'group-name';
      nameEl.textContent = groupData.name || 'Unknown Group';

      if (isPinned) {
        const pinBadge = document.createElement('span');
        pinBadge.className = 'group-pin-badge';
        pinBadge.title = 'Pinned to Top';
        pinBadge.innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M16 12V4H17V2H7V4H8V12L6 14V16H11V22H13V16H18V14L16 12Z"/></svg>';
        nameEl.appendChild(pinBadge);
      }

      // Count badge
      const countEl = document.createElement('span');
      countEl.className = 'group-count';
      countEl.textContent = servers.length;

      // Actions
      const actionsEl = document.createElement('div');
      actionsEl.className = 'group-actions';

      // Only show actions for subscription groups (not manual)
      if (groupKey !== 'manual') {
        const optionsBtn = document.createElement('button');
        optionsBtn.className = 'group-action-btn';
        optionsBtn.title = 'Options';
        optionsBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1.5"></circle><circle cx="19" cy="12" r="1.5"></circle><circle cx="5" cy="12" r="1.5"></circle></svg>';

        const dropdownMenu = document.createElement('div');
        dropdownMenu.className = 'group-dropdown-menu';

        // Pin/Unpin Item
        const pinItem = document.createElement('button');
        pinItem.className = 'dropdown-item';
        pinItem.innerHTML = isPinned
          ? '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="2" y1="2" x2="22" y2="22"></line><path d="M12 17v5M9 9l-3 3v3h12v-3l-3-3M15 4a2 2 0 0 1 2 2v2H7V6a2 2 0 0 1 2-2h6z"></path></svg> Unpin'
          : '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 17v5M9 9l-3 3v3h12v-3l-3-3M15 4a2 2 0 0 1 2 2v2H7V6a2 2 0 0 1 2-2h6z"></path></svg> Pin to Top';

        pinItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          togglePinSubscription(groupKey);
        });

        const pingItem = document.createElement('button');
        pingItem.className = 'dropdown-item';
        pingItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="2"></circle><path d="M16.24 7.76a6 6 0 0 1 0 8.49m-8.48-.01a6 6 0 0 1 0-8.49m11.31-2.82a10 10 0 0 1 0 14.14m-14.14 0a10 10 0 0 1 0-14.14"></path></svg> Ping Group';
        pingItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          pingGroup(groupKey);
        });

        const updateItem = document.createElement('button');
        updateItem.className = 'dropdown-item';
        updateItem.id = `group-update-${hashCode(groupKey)}`;
        updateItem.innerHTML = '<svg class="update-svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><polyline points="1 20 1 14 7 14"></polyline><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg> Update';
        updateItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          updateSingleSubscription(groupKey);
        });

        const deleteItem = document.createElement('button');
        deleteItem.className = 'dropdown-item delete';
        deleteItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg> Delete';
        deleteItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          deleteSubscription(groupKey);
        });

        dropdownMenu.appendChild(pinItem);
        dropdownMenu.appendChild(pingItem);
        dropdownMenu.appendChild(updateItem);
        dropdownMenu.appendChild(deleteItem);

        optionsBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          document.querySelectorAll('.group-dropdown-menu.show').forEach(el => {
            if (el !== dropdownMenu) el.classList.remove('show');
          });
          dropdownMenu.classList.toggle('show');
        });

        actionsEl.appendChild(optionsBtn);
        header.appendChild(dropdownMenu);
      }

      header.appendChild(chevron);
      header.appendChild(nameEl);
      header.appendChild(countEl);
      header.appendChild(actionsEl);

      // Click header to toggle collapse
      header.addEventListener('click', (e) => {
        if (isDraggingGroup) return;
        collapsedGroups[groupKey] = !collapsedGroups[groupKey];
        groupEl.classList.toggle('collapsed');
      });

      groupEl.appendChild(header);

      // Servers container
      const serversContainer = document.createElement('div');
      serversContainer.className = 'group-servers';

      for (const s of servers) {
        const item = createServerItem(s, groupKey);
        serversContainer.appendChild(item);
      }

      groupEl.appendChild(serversContainer);
      serverList.appendChild(groupEl);
    }

    // Add dragover to the server list container itself to prevent cross cursor in gaps
    serverList.addEventListener('dragenter', (e) => {
      if (draggedGroupKey) e.preventDefault();
    });
    serverList.addEventListener('dragover', (e) => {
      if (draggedGroupKey) {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'move';
      }
    });

  } catch (e) {
    log('Failed to load servers: ' + e);
  }
}

function createServerItem(serverData, groupKey) {
  const link = serverData.link;
  const remark = serverData.remark || 'Server';
  const latency = serverData.latency;

  const item = document.createElement('div');
  item.className = 'server-item';
  item.dataset.link = link;
  item.dataset.group = groupKey;

  if (link === selectedServerLink) {
    item.classList.add('selected');
  }
  if (link === connectedServerLink) {
    item.classList.add('connected');
  }

  // Flag
  const flagEl = document.createElement('div');
  flagEl.className = 'server-flag';
  const flagEmoji = extractFlagEmoji(remark);
  if (flagEmoji) {
    const code = emojiToCountryCode(flagEmoji);
    if (code) {
      const img = document.createElement('img');
      img.src = `https://flagcdn.com/w40/${code}.png`;
      img.alt = code;
      img.onerror = () => { img.replaceWith(document.createTextNode('🌐')); };
      flagEl.appendChild(img);
    } else {
      flagEl.textContent = flagEmoji;
    }
  } else {
    flagEl.textContent = '🌐';
  }

  // Name
  const nameEl = document.createElement('div');
  nameEl.className = 'server-name';
  nameEl.textContent = remark;

  // Latency
  const latencyEl = document.createElement('div');
  latencyEl.className = 'server-latency';
  latencyEl.id = `latency-${hashCode(link)}`;
  updateLatencyDisplay(latencyEl, latency);

  // Action Buttons
  const actionsEl = document.createElement('div');
  actionsEl.className = 'server-actions';

  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="2"></circle><path d="M16.24 7.76a6 6 0 0 1 0 8.49m-8.48-.01a6 6 0 0 1 0-8.49m11.31-2.82a10 10 0 0 1 0 14.14m-14.14 0a10 10 0 0 1 0-14.14"></path></svg>', () => pingSingleItem(link)));
  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="3" ry="3"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>', () => copyToClipboard(link)));
  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>', () => deleteServer(groupKey, link), true));

  // Click to select
  item.addEventListener('click', (e) => {
    if (e.target.closest('.server-action-btn')) return;
    selectServer(link);
  });

  item.appendChild(flagEl);
  item.appendChild(nameEl);
  item.appendChild(latencyEl);
  item.appendChild(actionsEl);

  return item;
}

function makeActionBtn(icon, callback, isDelete = false) {
  const btn = document.createElement('button');
  btn.className = 'server-action-btn' + (isDelete ? ' delete' : '');
  btn.innerHTML = icon;
  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    callback();
  });
  return btn;
}

function selectServer(link) {
  selectedServerLink = link;
  document.querySelectorAll('.server-item').forEach(el => {
    el.classList.toggle('selected', el.dataset.link === link);
  });
  updateConnectBtnText();
}

// ===== Connection =====
async function toggleConnection() {
  if (!selectedServerLink) {
    log('Please select a server first.');
    return;
  }
  
  if (isInvoking) {
    log('Please wait, processing...');
    return;
  }

  isInvoking = true;
  updateConnectBtnText();

  try {
    if (isRunning) {
      if (connectedServerLink === selectedServerLink) {
        await disconnect();
      } else {
        log('Switching server...');
        await connectToLink(selectedServerLink, true);
      }
    } else {
      await connectToLink(selectedServerLink, false);
    }
  } finally {
    isInvoking = false;
    updateConnectBtnText();
  }
}

async function connectToLink(link, seamless) {
  if (connectedServerLink && connectedServerLink !== link) {
    const oldItem = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
    if (oldItem) oldItem.classList.remove('connected', 'processing');
  }

  connectedServerLink = link;
  log(`Configuring core for: ${link.substring(0, 40)}...`);

  const tunMode = tunToggle.checked;

  isRunning = true;
  isConnecting = true;
  isTunRouted = !tunMode;
  connectStartTime = Date.now();
  updateConnectBtnText();
  
  statusLabel.textContent = 'Processing...';
  statusLabel.className = 'status processing';
  
  livePing.textContent = 'Ping: -';
  livePing.className = 'live-ping';
  livePing.style.display = 'block';

  if (!seamless) {
    flagImg.style.display = 'none';
  }
  
  const item = document.querySelector(`.server-item[data-link="${link}"]`);
  if (item) {
    item.classList.remove('connected', 'failed');
    item.classList.add('processing');
  }

  if (tunMode) {
    try {
      const admin = await invoke('is_admin');
      if (!admin) {
        tunToggle.checked = false;
        await invoke('toggle_tun', { enabled: false });
        
        const overlay = document.createElement('div');
        overlay.style.position = 'fixed';
        overlay.style.top = '0'; overlay.style.left = '0';
        overlay.style.width = '100vw'; overlay.style.height = '100vh';
        overlay.style.backgroundColor = 'rgba(0,0,0,0.85)';
        overlay.style.color = 'white';
        overlay.style.display = 'flex';
        overlay.style.alignItems = 'center';
        overlay.style.justifyContent = 'center';
        overlay.style.zIndex = '9999';
        overlay.style.backdropFilter = 'blur(4px)';
        overlay.innerHTML = '<div style="text-align:center; padding: 20px; border-radius: 12px; background: var(--bg-secondary);"><h3>Administrator Required</h3><p style="margin-top: 10px; color: var(--text-secondary);">Restarting Vxray as Administrator to enable TUN Mode...</p></div>';
        document.body.appendChild(overlay);
        setTimeout(() => invoke('restart_as_admin'), 3000);
        await disconnect();
        return;
      }
    } catch (_) {}
  }

  try {
    const result = await invoke('connect', { link, tunMode });
    if (result === null || result === undefined || result === 'ok' || result === true) {
      setTimeout(() => checkIp(!tunMode), 1000);
      try { await invoke('save_last_connection', { link, tunMode }); } catch (_) {}
    } else {
      log('Connection failed: ' + result);
      forceShowLogs();
      const item = document.querySelector(`.server-item[data-link="${link}"]`);
      if (item) {
        item.classList.remove('processing', 'connected');
        item.classList.add('failed');
      }
      await disconnect();
    }
  } catch (e) {
    log('Error: ' + e);
    forceShowLogs();
    const item = document.querySelector(`.server-item[data-link="${link}"]`);
    if (item) {
      item.classList.remove('processing', 'connected');
      item.classList.add('failed');
    }
    await disconnect();
  }
}

async function disconnect() {
  try {
    await invoke('disconnect');
    await invoke('save_last_connection', { link: "", tunMode: false });
  } catch (_) {}

  isRunning = false;
  isConnecting = false;
  
  if (connectedServerLink) {
    const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
    if (item) {
      item.classList.remove('connected', 'processing');
    }
  }
  
  connectedServerLink = null;
  consecutivePingFailures = 0;

  statusLabel.textContent = 'Offline';
  statusLabel.className = 'status offline';

  livePing.style.display = 'none';

  flagImg.style.display = 'none';
  ipAddress.textContent = '---.---.---.---';

  checkIp(false);
  updateConnectBtnText();
}

function updateConnectBtnText() {
  if (!selectedServerLink) return;
  const textSpan = btnConnect.querySelector('.connect-text');
  if (!textSpan) return;

  if (isRunning) {
    if (selectedServerLink === connectedServerLink) {
      textSpan.textContent = 'Disconnect';
      btnConnect.classList.add('disconnect');
    } else {
      textSpan.textContent = 'Switch Server';
      btnConnect.classList.remove('disconnect');
    }
  } else {
    textSpan.textContent = 'Connect';
    btnConnect.classList.remove('disconnect');
  }
}

// ===== IP Check =====
async function checkIp(useProxy) {
  if (useProxy && !isRunning) return;

  try {
    const data = await invoke('check_ip', { useProxy });
    const parsed = typeof data === 'string' ? JSON.parse(data) : data;

    if (parsed.error) {
      if (isRunning && isConnecting) {
        if (Date.now() - connectStartTime > 6000) {
          log('Connection timeout: Server is unreachable.');
          forceShowLogs();
          if (connectedServerLink) {
            const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
            if (item) {
              item.classList.remove('processing', 'connected');
              item.classList.add('failed');
            }
          }
          return;
        }
        setTimeout(() => checkIp(true), 1000);
        return;
      }
      countryName.textContent = 'Failed';
      flagImg.style.display = 'none';
      return;
    }

    if (!isRunning && !originalIp && parsed.query) {
      originalIp = parsed.query;
    }

    if (isRunning && isConnecting && !useProxy) {
      if (originalIp && parsed.query === originalIp) {
        if (Date.now() - connectStartTime > 10000) {
          log('Connection timeout: TUN interface failed to route traffic.');
          forceShowLogs();
          if (connectedServerLink) {
            const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
            if (item) {
              item.classList.remove('processing', 'connected');
              item.classList.add('failed');
            }
          }
          return;
        }
        setTimeout(() => checkIp(false), 1000);
        return;
      }
      if (!isTunRouted) {
        isTunRouted = true;
        runLivePing();
      }
    }

    if (isRunning) {
      if (isConnecting && isTunRouted) {
        runLivePing();
      }
    } else {
      statusLabel.textContent = 'Online';
      statusLabel.className = 'status online';
    }

    countryName.textContent = parsed.country || 'Unknown';
    ipAddress.textContent = parsed.query || 'Unknown IP';

    const code = (parsed.countryCode || '').toLowerCase();
    if (code) {
      flagImg.src = `https://flagcdn.com/w40/${code}.png`;
      flagImg.style.display = 'inline';
    }
  } catch (e) {
    if (isRunning && isConnecting) {
      if (Date.now() - connectStartTime > 20000) {
        log('Connection timeout: Server is unreachable.');
        forceShowLogs();
        if (connectedServerLink) {
          const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
          if (item) {
            item.classList.remove('processing', 'connected');
            item.classList.add('failed');
          }
        }
        return;
      }
      setTimeout(() => checkIp(!tunToggle.checked), 1000);
    }
  }
}

// ===== Live Ping =====
async function runLivePing() {
  if (!isRunning || !connectedServerLink) {
    updateLivePingLabel(null);
    return;
  }

  try {
    const latency = await invoke('get_live_ping', { link: connectedServerLink });
    updateLivePingLabel(latency);
  } catch (_) {
    updateLivePingLabel(null);
  }
}

function updateLivePingLabel(latency) {
  if (!isRunning) {
    livePing.style.display = 'none';
    consecutivePingFailures = 0;
    return;
  }

  if (latency === null || latency === 'Timeout' || latency === undefined) {
    livePing.textContent = 'Ping: Timeout';
    livePing.className = 'live-ping red';
    
    consecutivePingFailures++;
    if (consecutivePingFailures >= 3) {
      log('Server unreachable (ping timeout).');
      if (connectedServerLink) {
        const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
        if (item) {
          item.classList.remove('processing', 'connected');
          item.classList.add('failed');
        }
      }
    }
    return;
  }

  consecutivePingFailures = 0;

  const ms = typeof latency === 'number' ? latency : parseInt(latency);
  livePing.style.display = 'block';
  livePing.textContent = `Ping: ${ms} ms`;

  if (ms < 500) {
    livePing.className = 'live-ping green';
  } else {
    livePing.className = 'live-ping yellow';
  }

  if (isRunning && isConnecting && isTunRouted) {
    isConnecting = false;
    statusLabel.textContent = 'Connected';
    statusLabel.className = 'status connected';
    
    if (connectedServerLink) {
      const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
      if (item) {
        item.classList.remove('processing');
        item.classList.add('connected');
      }
    }
  }
}

// ===== Crash Monitor =====
async function checkCoreHealth() {
  if (!isRunning) return;

  try {
    const alive = await invoke('is_core_alive');
    if (!alive) {
      log('CRASH DETECTED: Sing-box core process terminated unexpectedly.');
      await disconnect();
    }
  } catch (_) {}
}

// ===== Latency Testing =====
let shouldStopTesting = false;

async function testAllLatency() {
  if (testingActive) {
    shouldStopTesting = true;
    btnTest.textContent = 'Stopping...';
    btnTest.disabled = true;
    return;
  }
  
  testingActive = true;
  shouldStopTesting = false;
  btnTest.textContent = 'Stop Testing';

  const links = [];
  document.querySelectorAll('.server-item').forEach(item => {
    links.push(item.dataset.link);
    const latEl = item.querySelector('.server-latency');
    if (latEl) updateLatencyDisplay(latEl, '...');
  });

  if (links.length === 0) {
    finishLatencyTest();
    return;
  }

  for (const link of links) {
    if (shouldStopTesting) {
      log('Latency testing stopped by user.');
      break;
    }

    updateLatencyUi(link, 'Testing...');
    try {
      const result = await invoke('test_single_latency', { link });
      const parsed = typeof result === 'string' ? JSON.parse(result) : result;
      const latency = parsed.latency !== undefined ? parsed.latency : parsed;
      updateLatencyUi(link, latency);
    } catch (e) {
      updateLatencyUi(link, 'Error');
    }
  }

  if (shouldStopTesting) {
    document.querySelectorAll('.server-item').forEach(item => {
      const latEl = item.querySelector('.server-latency');
      if (latEl && (latEl.textContent === '...' || latEl.textContent === 'Testing...')) {
        updateLatencyDisplay(latEl, '');
      }
    });
  }

  finishLatencyTest();
}

function finishLatencyTest() {
  testingActive = false;
  shouldStopTesting = false;
  btnTest.disabled = false;
  btnTest.textContent = 'Test Latency';
}

async function pingSingleItem(link) {
  if (testingActive) return;
  testingActive = true;
  updateLatencyUi(link, '...');

  try {
    const result = await invoke('test_single_latency', { link });
    const parsed = typeof result === 'string' ? JSON.parse(result) : result;
    const latency = parsed.latency !== undefined ? parsed.latency : parsed;
    updateLatencyUi(link, latency);
  } catch (e) {
    updateLatencyUi(link, 'Error');
  }

  testingActive = false;
}

function updateLatencyUi(link, latency) {
  const hash = hashCode(link);
  const el = document.getElementById(`latency-${hash}`);
  if (el) updateLatencyDisplay(el, latency);
}

function updateLatencyDisplay(el, latency) {
  if (latency === null || latency === undefined || latency === '') {
    el.textContent = '';
    el.className = 'server-latency';
    return;
  }

  if (typeof latency === 'number') {
    el.textContent = `${latency} ms`;
    if (latency < 500) {
      el.className = 'server-latency green';
    } else {
      el.className = 'server-latency yellow';
    }
  } else {
    el.textContent = String(latency);
    if (latency === '...') {
      el.className = 'server-latency';
    } else {
      el.className = 'server-latency red';
    }
  }
}

// ===== Settings =====
async function handleSmartAdd() {
  const input = document.getElementById('txt-input');
  const text = input.value.trim();
  if (!text) return;

  const proxyProtocols = ['vless://', 'vmess://', 'ss://', 'trojan://', 'hy2://', 'hysteria2://', 'tuic://'];
  const isProxy = proxyProtocols.some(p => text.startsWith(p));

  if (text.startsWith('http') && text.includes('://') && !isProxy) {
    // Subscription URL
    try {
      await invoke('add_subscription', { url: text });
      input.value = '';
      await updateAllSubscriptions();
    } catch (e) {
      log('Failed to add subscription: ' + e);
    }
  } else if (text.includes('://')) {
    // Single server link
    try {
      await invoke('add_server', { link: text });
      input.value = '';
      await refreshServerList();
      switchTab(0);
    } catch (e) {
      log('Failed to add server: ' + e);
    }
  }
}

async function updateAllSubscriptions() {
  const btn = document.getElementById('btn-update-subs');
  btn.disabled = true;
  btn.textContent = 'Updating...';

  try {
    const subsStr = await invoke('get_subscriptions');
    const subs = typeof subsStr === 'string' ? JSON.parse(subsStr) : subsStr;

    for (const sub of subs) {
      try {
        await invoke('update_subscription', { url: sub.url });
      } catch (e) {
        log('Failed to update ' + sub.url + ': ' + e);
      }
    }
    await refreshServerList();
    log('Subscriptions updated.');
  } catch (e) {
    log('Update failed: ' + e);
  }

  btn.disabled = false;
  btn.textContent = 'Update All Subscriptions';
}

async function deleteServer(groupKey, link) {
  try {
    await invoke('delete_server', { group: groupKey, link });
    if (selectedServerLink === link) {
      selectedServerLink = null;
    }
    await refreshServerList();
  } catch (e) {
    log('Delete failed: ' + e);
  }
}

async function deleteSubscription(groupKey) {
  try {
    await invoke('delete_subscription', { groupKey });
    log('Subscription removed.');
    await refreshServerList();
  } catch (e) {
    log('Delete subscription failed: ' + e);
  }
}

async function updateSingleSubscription(groupKey) {
  const btnId = `group-update-${hashCode(groupKey)}`;
  const btn = document.getElementById(btnId);
  if (btn) btn.classList.add('updating');

  try {
    await invoke('update_subscription', { url: groupKey });
    await refreshServerList();
    log('Subscription updated: ' + (groupKey.split('/')[2] || groupKey));
  } catch (e) {
    log('Update failed: ' + e);
  }

  if (btn) btn.classList.remove('updating');
}

async function pingGroup(groupKey) {
  if (testingActive) return;
  testingActive = true;

  const groupEl = document.querySelector(`.sub-group[data-group-key="${groupKey}"]`);
  if (!groupEl) { testingActive = false; return; }

  // Expand if collapsed
  if (groupEl.classList.contains('collapsed')) {
    collapsedGroups[groupKey] = false;
    groupEl.classList.remove('collapsed');
  }

  const items = groupEl.querySelectorAll('.server-item');
  const links = [];
  items.forEach(item => {
    links.push(item.dataset.link);
    const latEl = item.querySelector('.server-latency');
    if (latEl) updateLatencyDisplay(latEl, '...');
  });

  for (const link of links) {
    updateLatencyUi(link, 'Testing...');
    try {
      const result = await invoke('test_single_latency', { link });
      const parsed = typeof result === 'string' ? JSON.parse(result) : result;
      const latency = parsed.latency !== undefined ? parsed.latency : parsed;
      updateLatencyUi(link, latency);
    } catch (e) {
      updateLatencyUi(link, 'Error');
    }
  }

  testingActive = false;
}

async function clearData() {
  try {
    await invoke('clear_data');
    selectedServerLink = null;
    await refreshServerList();
    log('All data cleared.');
  } catch (e) {
    log('Clear failed: ' + e);
  }
}

// ===== TUN Toggle =====
async function handleTunToggle() {
  const checked = tunToggle.checked;

  if (checked) {
    try {
      const admin = await invoke('is_admin');
      if (!admin) {
        tunToggle.checked = false;
        await invoke('toggle_tun', { enabled: false });
        
        const overlay = document.createElement('div');
        overlay.style.position = 'fixed';
        overlay.style.top = '0'; overlay.style.left = '0';
        overlay.style.width = '100vw'; overlay.style.height = '100vh';
        overlay.style.backgroundColor = 'rgba(0,0,0,0.85)';
        overlay.style.color = 'white';
        overlay.style.display = 'flex';
        overlay.style.alignItems = 'center';
        overlay.style.justifyContent = 'center';
        overlay.style.zIndex = '9999';
        overlay.style.backdropFilter = 'blur(4px)';
        overlay.innerHTML = '<div style="text-align:center; padding: 20px; border-radius: 12px; background: var(--bg-secondary);"><h3>Administrator Required</h3><p style="margin-top: 10px; color: var(--text-secondary);">Restarting Vxray as Administrator to enable TUN Mode...</p></div>';
        document.body.appendChild(overlay);
        
        setTimeout(() => invoke('restart_as_admin'), 3000);
        return;
      }
    } catch (_) {}
  }

  await invoke('toggle_tun', { enabled: checked });

  // If connected, reconnect with new mode
  if (isRunning && connectedServerLink) {
    const modeStr = checked ? 'TUN' : 'System Proxy';
    log(`Seamlessly switching to ${modeStr} mode...`);
    await connectToLink(connectedServerLink, true);
  }
}

// ===== Logs =====
function toggleLogs() {
  logsOpen = !logsOpen;
  logViewer.classList.toggle('open', logsOpen);
  btnLogs.textContent = logsOpen ? 'Hide Logs' : 'Show Logs';
}

function forceShowLogs() {
  logsOpen = true;
  logViewer.classList.add('open');
  btnLogs.textContent = 'Hide Logs';
}

function log(msg) {
  const line = document.createElement('div');
  line.textContent = msg;
  logViewer.appendChild(line);
  logViewer.scrollTop = logViewer.scrollHeight;
}

// ===== Clipboard =====
async function copyToClipboard(text) {
  try {
    await navigator.clipboard.writeText(text);
  } catch (_) {
    // Fallback
    const ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
  }
}

// ===== Helpers =====
function extractFlagEmoji(text) {
  const match = text.match(/[\uD83C][\uDDE6-\uDDFF][\uD83C][\uDDE6-\uDDFF]/);
  return match ? match[0] : null;
}

function emojiToCountryCode(emoji) {
  try {
    const codePoints = [...emoji].map(c => c.codePointAt(0));
    return codePoints.map(cp => String.fromCharCode(cp - 0x1F1E6 + 0x61)).join('');
  } catch (_) {
    return null;
  }
}

function hashCode(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32-bit integer
  }
  return Math.abs(hash);
}

// ===== Global Events =====
document.addEventListener('click', (e) => {
  // Close any open dropdown menus if clicking outside
  if (!e.target.closest('.group-dropdown-menu') && !e.target.closest('.group-action-btn')) {
    document.querySelectorAll('.group-dropdown-menu.show').forEach(el => {
      el.classList.remove('show');
    });
  }
});

// ===== Keyboard shortcut =====
document.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && document.activeElement?.id === 'txt-input') {
    handleSmartAdd();
  }
});
