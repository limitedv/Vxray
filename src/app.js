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
let baseIp = null;
let isTunRouted = false;
let isDraggingGroup = false;
let isDraggingServer = false;
let draggedServerLink = null;
let draggedServerGroup = null;

// ===== DOM Refs =====
const serverList = document.getElementById('server-list');
const logViewer = document.getElementById('log-viewer');
const btnConnect = document.getElementById('btn-connect');
const btnTest = document.getElementById('btn-test');
const btnLogs = document.getElementById('btn-logs');
const statusLabel = document.getElementById('status-label');

function animateTextChange(element, newText) {
  if (!element || element.textContent === newText) return;
  element.style.transition = 'opacity 0.2s ease-in-out';
  element.style.opacity = '0';
  setTimeout(() => {
    element.textContent = newText;
    element.style.opacity = '1';
    if (newText.length > 15) element.title = newText;
  }, 200);
}

function animateIpAndFlag(newIp, newFlagSrc) {
  const ipAddress = document.getElementById('ip-address');
  const flagImg = document.getElementById('flag-img');
  if (!ipAddress || !flagImg) return;

  if (ipAddress.textContent === newIp && flagImg.src === newFlagSrc) return;

  // Cleanup old transitions so they don't fight with Web Animations API
  ipAddress.style.transition = 'none';
  flagImg.style.transition = 'none';

  const animOut = [
    { opacity: 1, transform: 'translateY(0) scale(1) rotateX(0deg)', filter: 'blur(0px)' },
    { opacity: 0, transform: 'translateY(-15px) scale(0.9) rotateX(45deg)', filter: 'blur(8px)' }
  ];
  
  const animIn = [
    { opacity: 0, transform: 'translateY(15px) scale(0.9) rotateX(-45deg)', filter: 'blur(8px)' },
    { opacity: 1, transform: 'translateY(0) scale(1) rotateX(0deg)', filter: 'blur(0px)' }
  ];
  
  const options = { duration: 600, easing: 'cubic-bezier(0.68, -0.55, 0.265, 1.55)', fill: 'forwards' };

  const p1 = ipAddress.animate(animOut, { ...options, duration: 400, easing: 'ease-in' }).finished;
  const p2 = flagImg.animate(animOut, { ...options, duration: 400, easing: 'ease-in' }).finished;

  Promise.all([p1, p2]).then(() => {
    ipAddress.textContent = newIp;
    if (newIp.length > 15) ipAddress.title = newIp;

    if (newFlagSrc) {
      flagImg.src = newFlagSrc;
      flagImg.style.display = 'inline';
    } else {
      flagImg.style.display = 'none';
      flagImg.src = '';
    }

    ipAddress.animate(animIn, options);
    flagImg.animate(animIn, options);
  }).catch(() => {});
}

function updateSmartDash(state, pingMs = null) {
  
  const dash = document.getElementById('lcd-network');
  const capsule = document.getElementById('lcd-capsule');

  const icon = document.getElementById('dash-icon');
  const title = document.getElementById('dash-title');
  const subtitle = document.getElementById('dash-subtitle');
  const rimPing = document.getElementById('live-ping-rim');
  
  if (!dash || !icon || !title || !subtitle) return;
  
  dash.classList.remove('offline', 'online', 'processing', 'connected', 'timeout');
  dash.classList.add(state);
  if (capsule) {
    capsule.classList.remove('offline', 'online', 'processing', 'connected', 'timeout');
    capsule.classList.add(state);
  }
  
  const IOS_SPINNER_HTML = `<div class="ios-spinner"><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div><div class="blade"></div></div>`;

  const setCapsuleBadge = (text, color = 'rgba(255,255,255,0.4)', bg = 'rgba(255,255,255,0.06)') => {
    if (rimPing) {
      rimPing.textContent = text;
      rimPing.style.color = color;
      rimPing.style.background = bg;
      rimPing.style.border = '1px solid rgba(255,255,255,0.08)';
      rimPing.style.boxShadow = 'inset 0 1px 2px rgba(0,0,0,0.25)';
      rimPing.style.minWidth = '48px';
    }
  };
  
  if (state === 'offline') {
    icon.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>';
    title.textContent = 'Offline';
    subtitle.textContent = 'Waiting for network...';
    setCapsuleBadge('OFF', 'rgba(255,255,255,0.3)');
  } else if (state === 'online') {
    icon.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>';
    title.textContent = 'System Ready';
    subtitle.textContent = 'Idle';
    setCapsuleBadge('READY', 'rgba(255,255,255,0.45)');
  } else if (state === 'processing') {
    icon.innerHTML = IOS_SPINNER_HTML;
    title.textContent = 'Processing...';
    subtitle.textContent = 'Connecting to server';
    if (rimPing) {
      rimPing.innerHTML = IOS_SPINNER_HTML;
      rimPing.style.background = 'rgba(255,255,255,0.08)';
      rimPing.style.border = '1px solid rgba(255,255,255,0.18)';
      rimPing.style.boxShadow = 'inset 0 1px 2px rgba(0,0,0,0.25), 0 0 14px rgba(255,255,255,0.18)';
      rimPing.style.minWidth = '48px';
    }
  } else if (state === 'timeout') {
    icon.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>';
    title.textContent = 'Timeout';
    
    const livePingToggle = document.getElementById('toggle-live-ping');
    const isPingEnabled = livePingToggle ? livePingToggle.checked : true;
    const tunSwitch = document.getElementById('tun-toggle');
    const isTun = tunSwitch ? tunSwitch.checked : false;
    
    subtitle.textContent = isTun ? 'Global Tunnel Active' : 'System Proxy Active';
    subtitle.style.color = 'var(--text-secondary)';

    setCapsuleBadge('ERR', '#fbbf24', 'rgba(251,191,36,0.1)');
  } else if (state === 'connected') {
    icon.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="pulse"><path d="M22 12h-4l-3 9L9 3l-3 9H2"></path></svg>';
    title.textContent = 'Connected';
    
    const livePingToggle = document.getElementById('toggle-live-ping');
    const isPingEnabled = livePingToggle ? livePingToggle.checked : true;
    const tunSwitch = document.getElementById('tun-toggle');
    const isTun = tunSwitch ? tunSwitch.checked : false;
    
    subtitle.textContent = isTun ? 'Global Tunnel Active' : 'System Proxy Active';
    subtitle.style.color = 'var(--text-secondary)';

    if (pingMs === 'Timeout') {
      setCapsuleBadge('ERR', '#ef4444', 'rgba(239,68,68,0.1)');
    } else if (!isPingEnabled) {
      setCapsuleBadge('LIVE', '#4ade80', 'rgba(74,222,128,0.1)');
    } else {
      if (pingMs === null) {
        if (rimPing && rimPing.textContent && rimPing.textContent.includes('ms')) {
          // Preserve the last known ping value during transient updates
        } else {
          setCapsuleBadge('PING', 'rgba(255,255,255,0.4)');
        }
      } else {
        const pingColor = pingMs < 500 ? '#4ade80' : '#fbbf24';
        setCapsuleBadge(pingMs + ' ms', pingColor, 'rgba(255,255,255,0.06)');
      }
    }
  }
}


const livePing = document.getElementById('live-ping');
const flagImg = document.getElementById('flag-img');
const locPrefix = document.getElementById('loc-prefix');
const countryName = document.getElementById('country-name');
const ipAddress = document.getElementById('ip-address');
const tunToggle = document.getElementById('tun-toggle');
const btnUpdateSubs = document.getElementById('btn-update-subs');
const toggleAutoReconnect = document.getElementById('toggle-auto-reconnect');
const toggleAutoStart = document.getElementById('toggle-auto-start');
const toggleStartMinimized = document.getElementById('toggle-start-minimized');
const rowStartMinimized = document.getElementById('row-start-minimized');

// ===== Initialization =====

function updateTunUIState() {
  const tunSwitch = document.getElementById('tun-toggle');
  if (!tunSwitch) return;
  
  const microCheckbox = document.getElementById('micro-tun-checkbox');
  if (microCheckbox) {
      microCheckbox.checked = tunSwitch.checked;
  }
}

function toggleMicroDrawer() {
  const sidebar = document.getElementById('sidebar');
  const drawer = document.getElementById('lcd-network');
  
  if (sidebar && sidebar.classList.contains('collapsed')) {
    sidebar.classList.remove('collapsed');
    if (drawer) {
      drawer.classList.add('open');
      localStorage.setItem('drawerOpen', 'true');
    }
    return;
  }
  
  if (drawer) {
    const isOpen = drawer.classList.toggle('open');
    localStorage.setItem('drawerOpen', isOpen ? 'true' : 'false');
  }
}

function toggleTunUI() {
  const tunSwitch = document.getElementById('tun-toggle');
  if (!tunSwitch) return;
  tunSwitch.checked = !tunSwitch.checked;
  const e = new Event('change');
  tunSwitch.dispatchEvent(e);
}

async function initApp() {
  checkIp(false);

  // Titlebar controls
  const minBtn = document.getElementById('titlebar-minimize');
  if (minBtn) minBtn.addEventListener('click', () => invoke('minimize_window'));

  const maxBtn = document.getElementById('titlebar-maximize');
  if (maxBtn) maxBtn.addEventListener('click', () => invoke('maximize_window'));

  const closeBtn = document.getElementById('titlebar-close');
  if (closeBtn) {
    closeBtn.addEventListener('mouseenter', () => closeBtn.classList.add('is-hovered'));
    closeBtn.addEventListener('mouseleave', () => closeBtn.classList.remove('is-hovered'));
    closeBtn.addEventListener('click', () => {
        closeBtn.classList.remove('is-hovered');
        invoke('close_window');
    });
  }

  // Restore drawer state from memory
  const savedDrawerState = localStorage.getItem('drawerOpen');
  if (savedDrawerState === 'true') {
    const drawer = document.getElementById('lcd-network');
    if (drawer) drawer.classList.add('open');
  }

  // Hover tracking for dock capsule shadow
  const dock = document.getElementById('lcd-network');
  if (dock) {
    dock.addEventListener('mousemove', (e) => {
      const rect = dock.getBoundingClientRect();
      dock.style.setProperty('--mouse-x', `${(e.clientX - rect.left).toFixed(1)}px`);
      dock.style.setProperty('--mouse-y', `${(e.clientY - rect.top).toFixed(1)}px`);
    });
  }

  // Hover tracking for connect button & tabs (dynamic island specular hover)
  const connectBtn = document.getElementById('btn-connect');
  if (connectBtn) {
    connectBtn.addEventListener('mousemove', (e) => {
      const rect = connectBtn.getBoundingClientRect();
      connectBtn.style.setProperty('--mouse-x', `${(e.clientX - rect.left).toFixed(1)}px`);
      connectBtn.style.setProperty('--mouse-y', `${(e.clientY - rect.top).toFixed(1)}px`);
    });
  }

  document.querySelectorAll('.tab-btn').forEach((tab) => {
    tab.addEventListener('mousemove', (e) => {
      const rect = tab.getBoundingClientRect();
      tab.style.setProperty('--mouse-x', `${(e.clientX - rect.left).toFixed(1)}px`);
      tab.style.setProperty('--mouse-y', `${(e.clientY - rect.top).toFixed(1)}px`);
    });
  });

  // Load TUN state
  try {
    const tunEnabled = await invoke('get_tun_enabled');
    if (tunToggle) { tunToggle.checked = tunEnabled; updateTunUIState(); }
      updateSmartDash('offline');
  } catch (_) {}

  // Load Startup Settings
  let startupSettings = null;
  try {
    startupSettings = await invoke('get_startup_settings');
    if (startupSettings) {
      if (toggleAutoReconnect) toggleAutoReconnect.checked = startupSettings.auto_reconnect;
      if (toggleAutoStart) toggleAutoStart.checked = startupSettings.auto_start;
      if (toggleStartMinimized) toggleStartMinimized.checked = startupSettings.start_minimized;
      if (startupSettings.auto_start && rowStartMinimized) {
        rowStartMinimized.classList.add('enabled');
      }
    }
  } catch (_) {}

  // Auto Reconnect Logic
  try {
    const info = await invoke('get_auto_reconnect_info');
    if (info.auto_reconnect && info.link && info.link !== "") {
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
  if (!startupSettings || startupSettings.auto_update_subs !== false) {
    setTimeout(updateAllSubscriptions, 2000);
  }

  // Start timers
  livePingInterval = setInterval(runLivePing, 4000);
  crashCheckInterval = setInterval(checkCoreHealth, 2000);

  // Global drag handling to absolutely prevent the "not allowed" cross cursor anywhere in the window
  document.addEventListener('dragenter', (e) => {
    if (isDraggingGroup || isDraggingServer) {
      e.preventDefault();
    }
  });
  
  document.addEventListener('dragover', (e) => {
    if (isDraggingGroup || isDraggingServer) {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
    }
  });

  document.addEventListener('drop', (e) => {
    if (isDraggingGroup || isDraggingServer) {
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
  document.getElementById('tab-subscriptions')?.classList.toggle('active', index === 1);
  document.getElementById('tab-routing')?.classList.toggle('active', index === 2);
  document.getElementById('tab-settings').classList.toggle('active', index === 3);
  document.getElementById('tab-speedtest')?.classList.toggle('active', index === 4);
  
  document.getElementById('page-servers').classList.toggle('active', index === 0);
  document.getElementById('page-subscriptions')?.classList.toggle('active', index === 1);
  document.getElementById('page-routing')?.classList.toggle('active', index === 2);
  document.getElementById('page-settings').classList.toggle('active', index === 3);
  document.getElementById('page-speedtest')?.classList.toggle('active', index === 4);

  // Only show log button on Servers tab
  const logBtn = document.getElementById('btn-logs');
  if (logBtn) {
    logBtn.style.display = index === 0 ? '' : 'none';
    if (index !== 0 && logsOpen) {
      logsOpen = false;
      logViewer.classList.remove('open');
      logBtn.classList.remove('active');
    }
  }

  if (index === 1) {
    refreshSubscriptions();
  }
}

// ===== Server List =====
let collapsedGroups = {};
let announcementOpenGroups = {};
let draggedGroupKey = null;

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

      let currentGhost = null;

      // Drag and drop event listeners
      header.addEventListener('dragstart', (e) => {
        if (e.target.closest('.group-dropdown-menu') || e.target.closest('.group-action-btn')) {
          e.preventDefault();
          return;
        }

        // Clear any accidental text selection so browser doesn't snapshot multi-line selections
        if (window.getSelection) {
          window.getSelection().removeAllRanges();
        }

        isDraggingGroup = true;
        draggedGroupKey = groupKey;

        // Create a crisp, single-item drag preview pill
        const ghost = document.createElement('div');
        ghost.className = 'drag-ghost-pill';

        const displayName = groupData.name || 'Subscription';
        const serverCount = servers.length;

        const iconSvg = groupKey === 'manual'
          ? '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="4" ry="4"></rect><rect x="2" y="14" width="20" height="8" rx="4" ry="4"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>'
          : '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>';

        ghost.innerHTML = `
          <div class="ghost-icon">${iconSvg}</div>
          <span class="ghost-title">${displayName}</span>
          ${isPinned ? '<span class="ghost-pin"><svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor"><path d="M16 12V4H17V2H7V4H8V12L6 14V16H11V22H13V16H18V14L16 12Z"/></svg></span>' : ''}
          <span class="ghost-badge">${serverCount}</span>
        `;

        document.body.appendChild(ghost);
        currentGhost = ghost;

        // Position drag preview neatly under the mouse pointer
        e.dataTransfer.setDragImage(ghost, 24, 22);

        setTimeout(() => {
          if (ghost.parentNode) ghost.remove();
        }, 50);

        // Defer adding the class so the browser captures the original look for the drag image
        setTimeout(() => {
          groupEl.classList.add('dragging');
        }, 0);
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', groupKey);
      });

      header.addEventListener('dragend', () => {
        if (currentGhost && currentGhost.parentNode) {
          currentGhost.remove();
          currentGhost = null;
        }
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

      // Hover-visible ping button on the header (available for both manual & subscription groups)
      const hoverPingBtn = document.createElement('button');
      hoverPingBtn.className = 'group-action-btn ping-all-btn';
      hoverPingBtn.title = 'Test Latency';
      hoverPingBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.55a11 11 0 0 1 14.08 0"></path><path d="M1.42 9a16 16 0 0 1 21.16 0"></path><path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path><line x1="12" y1="20" x2="12.01" y2="20"></line></svg>';
      let groupPingActive = false;
      hoverPingBtn.addEventListener('click', async (e) => {
        e.stopPropagation();
        if (groupPingActive) {
          // Stop
          shouldStopTesting = true;
          invoke('cancel_latency_tests').catch(()=>{});
          hoverPingBtn.title = 'Test Latency';
          hoverPingBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.55a11 11 0 0 1 14.08 0"></path><path d="M1.42 9a16 16 0 0 1 21.16 0"></path><path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path><line x1="12" y1="20" x2="12.01" y2="20"></line></svg>';
          groupPingActive = false;
          
          const gEl = document.querySelector(`.sub-group[data-group-key="${groupKey}"]`);
          if (gEl) {
            gEl.querySelectorAll('.server-latency').forEach(latEl => {
              if (latEl.textContent === '...' || latEl.textContent === 'Testing...') {
                updateLatencyDisplay(latEl, '');
              }
            });
          }
          return;
        }
        groupPingActive = true;
        hoverPingBtn.title = 'Stop';
        hoverPingBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="6" width="12" height="12"></rect></svg>';
        await pingGroup(groupKey);
        hoverPingBtn.title = 'Test Latency';
        hoverPingBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.55a11 11 0 0 1 14.08 0"></path><path d="M1.42 9a16 16 0 0 1 21.16 0"></path><path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path><line x1="12" y1="20" x2="12.01" y2="20"></line></svg>';
        groupPingActive = false;
      });

      const optionsBtn = document.createElement('button');
      optionsBtn.className = 'group-action-btn';
      optionsBtn.title = 'Options';
      optionsBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1.5"></circle><circle cx="19" cy="12" r="1.5"></circle><circle cx="5" cy="12" r="1.5"></circle></svg>';

      const dropdownMenu = document.createElement('div');
      dropdownMenu.className = 'group-dropdown-menu';

      // View Statistic (Both)
      const statsItem = document.createElement('button');
      statsItem.className = 'dropdown-item';
      statsItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 20V10M12 20V4M6 20v-6"></path></svg> View Statistic';
      statsItem.addEventListener('click', (e) => {
        e.stopPropagation();
        dropdownMenu.classList.remove('show');
        openStatsModal(groupKey, true, groupData.name || (groupKey === 'manual' ? 'Manually Added' : groupKey), groupData);
      });

      // Pin/Unpin Item (Both)
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

      if (groupKey === 'manual') {
        // Clear all manual servers
        const deleteItem = document.createElement('button');
        deleteItem.className = 'dropdown-item delete';
        deleteItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg> Delete All Servers';
        deleteItem.addEventListener('click', async (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          try {
            await invoke('clear_manual_servers');
            log('All manual servers deleted.');
            await refreshServerList();
          } catch (err) {
            log('Failed to clear manual servers: ' + err);
          }
        });

        dropdownMenu.appendChild(statsItem);
        dropdownMenu.appendChild(pinItem);
        dropdownMenu.appendChild(deleteItem);
      } else {
        // Edit Subscription
        const editItem = document.createElement('button');
        editItem.className = 'dropdown-item';
        editItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg> Edit';
        editItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          openEditSubModal(groupKey, groupData);
        });
        
        // Share Link
        const shareItem = document.createElement('button');
        shareItem.className = 'dropdown-item';
        shareItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"></circle><circle cx="6" cy="12" r="3"></circle><circle cx="18" cy="19" r="3"></circle><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line></svg> Share Link';
        shareItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          copyToClipboard(groupKey);
        });

        // Auto Update Toggle
        const autoUpdateItem = document.createElement('button');
        autoUpdateItem.className = 'dropdown-item';
        const isAutoUpdate = groupData.auto_update || false;
        autoUpdateItem.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15A9 9 0 0 1 3.51 9 9 9 0 0 1 20.49 15z"></path></svg> Auto Update <span class="toggle-badge' + (isAutoUpdate ? ' active' : '') + '">' + (isAutoUpdate ? 'ON' : 'OFF') + '</span>';
        autoUpdateItem.addEventListener('click', (e) => {
          e.stopPropagation();
          dropdownMenu.classList.remove('show');
          invoke('toggle_auto_update', { groupKey: groupKey, enabled: !isAutoUpdate }).then(() => refreshServerList()).catch(e => console.error(e));
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

        dropdownMenu.appendChild(editItem);
        dropdownMenu.appendChild(statsItem);
        dropdownMenu.appendChild(shareItem);
        dropdownMenu.appendChild(autoUpdateItem);
        dropdownMenu.appendChild(pinItem);
        dropdownMenu.appendChild(updateItem);
        dropdownMenu.appendChild(deleteItem);
      }

      optionsBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        document.querySelectorAll('.group-dropdown-menu.show').forEach(el => {
          if (el !== dropdownMenu) el.classList.remove('show');
        });
        dropdownMenu.classList.toggle('show');
      });

      actionsEl.appendChild(optionsBtn);
      header.appendChild(dropdownMenu);

      header.appendChild(chevron);
      const infoContainer = document.createElement('div');
      infoContainer.className = 'group-info';
      infoContainer.appendChild(nameEl);
      header.appendChild(infoContainer);

      

      

      if (groupKey !== 'manual' && groupData.total && groupData.total > 0) {
        const used = (groupData.upload || 0) + (groupData.download || 0);
        const remaining = groupData.total - used;
        const remainingPct = Math.max(0, Math.min(100, Math.round((remaining / groupData.total) * 100)));
        
        const wrapper = document.createElement('div');
        wrapper.className = 'group-traffic-ring-wrapper';
        
        const text = document.createElement('div');
        text.className = 'group-traffic-ring-text';
        text.textContent = formatBytes(remaining > 0 ? remaining : 0) + ' / ' + formatBytes(groupData.total);
        
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('viewBox', '-4 -4 44 44');
        svg.setAttribute('class', 'circular-chart');
        
        const pathBg = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        pathBg.setAttribute('class', 'circle-bg');
        pathBg.setAttribute('d', 'M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831');
        
        const pathFill = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        const colorClass = remainingPct <= 20 ? 'critical' : remainingPct <= 50 ? 'warning' : 'success';
        pathFill.setAttribute('class', 'circle-fill ' + colorClass);
        pathFill.setAttribute('stroke-dasharray', remainingPct + ', 100');
        pathFill.setAttribute('d', 'M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831');
        
        svg.appendChild(pathBg);
        svg.appendChild(pathFill);
        
        wrapper.appendChild(text);
        wrapper.appendChild(svg);
        
        header.appendChild(wrapper);
      }

      header.appendChild(countEl);
      if (hoverPingBtn) { header.appendChild(hoverPingBtn); }
      header.appendChild(actionsEl);

      // Click header to toggle collapse
      header.addEventListener('click', (e) => {
        if (isDraggingGroup || isDraggingServer) return;
        collapsedGroups[groupKey] = !collapsedGroups[groupKey];
        groupEl.classList.toggle('collapsed');
        saveCollapsedGroupsToSettings();
      });

      groupEl.appendChild(header);

      // Subscription announcement drawer & interactive zabooneh (pull-tab)
      if (groupData.announcement && groupData.announcement.trim()) {
        groupEl.classList.add('has-announcement');

        const assembly = document.createElement('div');
        assembly.className = 'subscription-drawer-assembly';

        // Check if user previously opened it, otherwise default to closed
        const isDrawerOpen = announcementOpenGroups[groupKey] || false;

        // 1. Drawer body (slides down from behind the subscription header)
        const drawerBody = document.createElement('div');
        drawerBody.className = 'subscription-drawer-body' + (isDrawerOpen ? '' : ' is-closed');
        drawerBody.addEventListener('click', (e) => e.stopPropagation());

        const content = document.createElement('div');
        content.className = 'announcement-content';

        const textEl = document.createElement('span');
        textEl.className = 'announcement-text';
        textEl.setAttribute('dir', 'auto');
        textEl.textContent = groupData.announcement.trim();
        textEl.title = groupData.announcement.trim();

        content.appendChild(textEl);
        drawerBody.appendChild(content);

        // 2. Zabooneh (Tongue / pull-tab at bottom of the drawer)
        const zaboonehWrapper = document.createElement('div');
        zaboonehWrapper.className = 'subscription-zabooneh-wrapper';

        const zabooneh = document.createElement('button');
        zabooneh.className = 'subscription-zabooneh' + (isDrawerOpen ? ' tab-open' : '');
        zabooneh.title = 'Toggle Announcement';
        zabooneh.innerHTML = `
          <span class="ann-tab-icon">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M11 6a13 13 0 0 0 8.4-2.8A1 1 0 0 1 21 4v12a1 1 0 0 1-1.6.8A13 13 0 0 0 11 14H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z"></path>
              <path d="M6 14a12 12 0 0 0 2.4 7.2 2 2 0 0 0 3.2-2.4A8 8 0 0 1 10 14"></path>
              <line x1="8" y1="6" x2="8" y2="14"></line>
            </svg>
          </span>
          <span class="ann-tab-label">Announcement</span>
          <span class="ann-tab-chevron">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </span>
        `;

        zabooneh.addEventListener('click', (e) => {
          e.stopPropagation();
          const willOpen = drawerBody.classList.contains('is-closed');
          if (willOpen) {
            drawerBody.classList.remove('is-closed');
            zabooneh.classList.add('tab-open');
            announcementOpenGroups[groupKey] = true;
          } else {
            drawerBody.classList.add('is-closed');
            zabooneh.classList.remove('tab-open');
            announcementOpenGroups[groupKey] = false;
          }
        });

        zaboonehWrapper.appendChild(zabooneh);
        assembly.appendChild(drawerBody);
        assembly.appendChild(zaboonehWrapper);
        groupEl.appendChild(assembly);
      }

      // Servers container
      const serversContainer = document.createElement('div');
      serversContainer.className = 'group-servers';

      serversContainer.addEventListener('dragenter', (e) => {
        if (isDraggingServer && draggedServerGroup === groupKey) e.preventDefault();
      });
      serversContainer.addEventListener('dragover', (e) => {
        if (isDraggingServer && draggedServerGroup === groupKey) {
          e.preventDefault();
          e.dataTransfer.dropEffect = 'move';
        }
      });

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
    if (consecutivePingFailures >= 4) {
      item.classList.add('timeout');
    } else if (isConnecting) {
      item.classList.add('processing');
    } else {
      item.classList.add('connected');
    }
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
      img.onerror = () => { flagEl.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="opacity:0.7"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>'; };
      flagEl.appendChild(img);
    } else {
      flagEl.textContent = flagEmoji;
    }
  } else {
    flagEl.innerHTML = '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="opacity:0.7"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>';
  }

  // Name
  const nameEl = document.createElement('div');
  nameEl.className = 'server-name';
  let cleanName = remark
    .replace(/[\uD83C][\uDDE6-\uDDFF][\uD83C][\uDDE6-\uDDFF]/g, '')
    .replace(/[\uD83C][\uDF0D-\uDF10]/g, '')
    .trim()
    .replace(/^[A-Z]{2}\b\s*[-|:]?\s*(?=\S)/, '')
    .replace(/^[-|:]\s*/, '')
    .trim();
  nameEl.textContent = cleanName || 'Server';

  // Latency
  const latencyEl = document.createElement('div');
  latencyEl.className = 'server-latency';
  latencyEl.id = `latency-${hashCode(link)}`;
  updateLatencyDisplay(latencyEl, latency);

  // Action Buttons
  const actionsEl = document.createElement('div');
  actionsEl.className = 'server-actions';

  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 20V10M12 20V4M6 20v-6"></path></svg>', () => openStatsModal(link, false, cleanName || 'Server', null)));
  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.55a11 11 0 0 1 14.08 0"></path><path d="M1.42 9a16 16 0 0 1 21.16 0"></path><path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path><line x1="12" y1="20" x2="12.01" y2="20"></line></svg>', () => pingSingleItem(link)));
  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="3" ry="3"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>', async () => {
    await copyToClipboard(link);
    showServerCopiedFeedback(item);
    log('Server link copied to clipboard.');
  }));
  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>', () => {
    openEditServerModal(groupKey, link);
  }));
  actionsEl.appendChild(makeActionBtn('<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>', () => deleteServer(groupKey, link), true));

  // Click to select
  item.addEventListener('click', (e) => {
    if (e.target.closest('.server-action-btn')) return;
    selectServer(link);
  });

  // Server Drag & Drop for playlist reordering
  let currentServerGhost = null;
  item.draggable = true;

  item.addEventListener('dragstart', (e) => {
    if (e.target.closest('.server-action-btn') || e.target.closest('.server-actions')) {
      e.preventDefault();
      return;
    }
    e.stopPropagation();

    if (window.getSelection) {
      window.getSelection().removeAllRanges();
    }

    isDraggingServer = true;
    draggedServerLink = link;
    draggedServerGroup = groupKey;

    // Create custom drag ghost for this single server
    const ghost = document.createElement('div');
    ghost.className = 'drag-ghost-server';
    
    const flagHtml = flagEl.innerHTML;
    const latencyText = latencyEl.textContent ? latencyEl.textContent.trim() : '';

    ghost.innerHTML = `
      <div class="ghost-flag">${flagHtml}</div>
      <span class="ghost-name">${cleanName || 'Server'}</span>
      ${latencyText ? `<span class="ghost-latency">${latencyText}</span>` : ''}
    `;

    document.body.appendChild(ghost);
    currentServerGhost = ghost;

    e.dataTransfer.setDragImage(ghost, 20, 21);

    setTimeout(() => {
      if (ghost.parentNode) ghost.remove();
    }, 50);

    setTimeout(() => {
      item.classList.add('dragging');
    }, 0);

    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', link);
  });

  item.addEventListener('dragend', () => {
    if (currentServerGhost && currentServerGhost.parentNode) {
      currentServerGhost.remove();
      currentServerGhost = null;
    }
    item.classList.remove('dragging');
    document.querySelectorAll('.server-item').forEach(el => {
      el.classList.remove('drag-target-above', 'drag-target-below');
    });
    setTimeout(() => {
      isDraggingServer = false;
      draggedServerLink = null;
      draggedServerGroup = null;
    }, 100);
  });

  item.addEventListener('dragover', (e) => {
    if (!isDraggingServer || draggedServerGroup !== groupKey) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = 'move';

    if (draggedServerLink === link) return;

    const rect = item.getBoundingClientRect();
    const midpoint = rect.top + rect.height / 2;

    document.querySelectorAll('.server-item').forEach(el => {
      el.classList.remove('drag-target-above', 'drag-target-below');
    });

    if (e.clientY < midpoint) {
      item.classList.add('drag-target-above');
    } else {
      item.classList.add('drag-target-below');
    }
  });

  item.addEventListener('dragleave', (e) => {
    if (!item.contains(e.relatedTarget)) {
      item.classList.remove('drag-target-above', 'drag-target-below');
    }
  });

  item.addEventListener('drop', async (e) => {
    if (!isDraggingServer || draggedServerGroup !== groupKey || !draggedServerLink) return;
    e.preventDefault();
    e.stopPropagation();

    item.classList.remove('drag-target-above', 'drag-target-below');

    if (draggedServerLink === link) return;

    const container = item.closest('.group-servers');
    if (!container) return;

    const draggedEl = Array.from(container.querySelectorAll('.server-item'))
      .find(el => el.dataset.link === draggedServerLink);

    if (!draggedEl || draggedEl === item) return;

    const rect = item.getBoundingClientRect();
    const midpoint = rect.top + rect.height / 2;
    const placeBefore = e.clientY < midpoint;

    if (placeBefore) {
      container.insertBefore(draggedEl, item);
    } else {
      container.insertBefore(draggedEl, item.nextSibling);
    }

    const orderedLinks = Array.from(container.querySelectorAll('.server-item'))
      .map(el => el.dataset.link);

    try {
      await invoke('save_server_order', { groupKey, orderedLinks });
    } catch (err) {
      console.error('Failed to save server order:', err);
    }
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
    // Clear failed state when user interacts
    if (el.dataset.link !== connectedServerLink) {
      el.classList.remove('failed', 'processing');
    }
  });
  updateConnectBtnText();
}

// ===== Connection =====
let isConnectToLinkActive = false;

async function toggleConnection() {
  if (!selectedServerLink) {
    log('Please select a server first.');
    return;
  }
  
  // ALWAYS allow forced disconnect if we are running or connecting to the selected server
  if ((isRunning || isConnectToLinkActive || isConnecting) && (connectedServerLink === selectedServerLink || !connectedServerLink)) {
    log('Force aborting connection...');
    isInvoking = true;
    updateConnectBtnText();
    try {
      await disconnect();
    } finally {
      isInvoking = false;
      updateConnectBtnText();
    }
    return;
  }

  if (isInvoking || isConnectToLinkActive) {
    log('Please wait, processing UI...');
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
  if (isConnectToLinkActive) {
    log('Connection in progress, ignoring click...');
    return;
  }
  isConnectToLinkActive = true;

  if (connectedServerLink && connectedServerLink !== link) {
      const oldItem = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
      if (oldItem) oldItem.classList.remove('connected', 'processing', 'timeout');
    }

    connectedServerLink = link;
  log(`Configuring core for: ${link.substring(0, 40)}...`);

  const tunMode = tunToggle.checked;

  isRunning = true;
  isConnecting = true;
  isTunRouted = false;
  connectStartTime = Date.now();
  updateConnectBtnText();
  
  updateSmartDash('processing');
  animateIpAndFlag('---.---.---.---', null);
  animateTextChange(document.getElementById('country-name'), 'Checking...');
    
    const item = document.querySelector(`.server-item[data-link="${link}"]`);
  if (item) {
    item.classList.remove('connected', 'failed', 'timeout');
    item.classList.add('processing');
  }

  if (tunMode) {
    try {
      const admin = await invoke('is_admin');
      if (!admin) {
          await invoke('toggle_tun', { enabled: true });
        
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
        isConnectToLinkActive = false;
        return;
      }
    } catch (_) {}
  }

    try {
        const result = await invoke('connect', { link, tunMode });
        
        if (!isRunning || connectedServerLink !== link) {
            log('Connection was aborted by user.');
            return;
        }

        if (result === null || result === undefined || result === 'ok' || result === true) {
          log('Connection established seamlessly.');
          
          // Instantly update UI to Connected for blazing fast feel
          isConnecting = false;
          isTunRouted = true;
          updateSmartDash('connected');
          
          const connectedItem = document.querySelector(`.server-item[data-link="${link}"]`);
          if (connectedItem) {
            connectedItem.classList.remove('processing', 'failed', 'timeout');
            connectedItem.classList.add('connected');
          }
          
          // Let IP checking run in background after a healthy delay to ensure old proxy socket is fully dead
          setTimeout(() => checkIp(true), 200);
      try { await invoke('save_last_connection', { link, tunMode }); } catch (_) {}
      
      // Start real live traffic tracking via clash API
      startTrafficTracking(link);
    } else {
      log('Connection failed: ' + result);
      forceShowLogs();
      const item = document.querySelector(`.server-item[data-link="${link}"]`);
      if (item) {
        item.classList.remove('processing', 'connected');
      }
      await disconnect();
    }
  } catch (e) {
    log('Error: ' + e);
    forceShowLogs();
    const item = document.querySelector(`.server-item[data-link="${link}"]`);
    if (item) {
      item.classList.remove('processing', 'connected');
    }
    await disconnect();
  } finally {
    isConnectToLinkActive = false;
  }
}

let trafficController = null;

async function startTrafficTracking(link) {
  if (trafficController) {
    trafficController.abort();
  }
  trafficController = new AbortController();
  
  // Helper: get local YYYY-MM-DD date string (not UTC)
  function getLocalDate() {
    const now = new Date();
    const y = now.getFullYear();
    const m = String(now.getMonth() + 1).padStart(2, '0');
    const d = String(now.getDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
  }
  
  let retryCount = 0;
  const maxRetries = 15; // Up to 15 seconds of retrying
  
  const connectAndRead = async () => {
    try {
      const response = await fetch('http://127.0.0.1:9090/traffic', { signal: trafficController.signal });
      retryCount = 0; // Connected successfully, reset
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      
      while (isRunning && connectedServerLink === link) {
        const { done, value } = await reader.read();
        if (done) break;
        const chunks = decoder.decode(value, { stream: true }).split('\n');
        for (const chunk of chunks) {
          if (!chunk.trim()) continue;
          try {
            const data = JSON.parse(chunk);
              const speed = data.up + data.down;
              if (speed > 0) {
                const date = getLocalDate();
                const item = Array.from(document.querySelectorAll('.server-item')).find(el => el.dataset.link === link);
                const group = item ? item.dataset.group : null;
                invoke('add_live_traffic', { link, group, speed, date }).then(res => {
                  if (res) {
                    // Update group (subscription) UI live
                    if (group) {
                      const hash = hashCode(group);
                      const usageEl = document.getElementById(`client-usage-${hash}`);
                      if (usageEl) {
                        usageEl.textContent = formatBytes(res.group_total);
                      }
                    }
                  }
                }).catch(console.error);
                
                // Update Smart Dash live speed
                const dashStats = document.getElementById('smart-dash-stats');
                if (dashStats) {
                  const currentTotalMatch = dashStats.textContent.match(/Total: (.*?)(?: \||$)/);
                  if (currentTotalMatch) {
                    // Visual estimation for dash, actual precise update happens periodically
                  }
                }
              }
          } catch (e) {}
        }
      }
    } catch (err) {
      if (err.name === 'AbortError' || !isRunning || connectedServerLink !== link) return;
      // Retry: the clash_api may not be ready yet (core startup delay)
      if (retryCount < maxRetries) {
        retryCount++;
        setTimeout(connectAndRead, 1000);
      }
    }
  };
  
  connectAndRead();
}

async function disconnect() {
  try {
    await invoke('disconnect');
    await invoke('save_last_connection', { link: "", tunMode: false });
  } catch (_) {}

  isRunning = false;
  isConnecting = false;
  isReconnecting = false;
  if (trafficController) trafficController.abort();
  
  if (connectedServerLink) {
    const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
    if (item) {
      item.classList.remove('connected', 'processing', 'timeout');
    }
  }
  
  connectedServerLink = null;
  consecutivePingFailures = 0;
  isConnectToLinkActive = false;

  // Instantly update the UI to offline/online state
  if (baseIp) {
    updateSmartDash('online');
    animateIpAndFlag(baseIp, null);
  } else {
    updateSmartDash('offline');
    animateIpAndFlag('---.---.---.---', null);
  }
  ipAddress.title = '';

  updateConnectBtnText();
  
  // Background task: check local IP and update flag/country
  invoke('check_ip', { useProxy: false }).then(data => {
    const parsed = typeof data === 'string' ? JSON.parse(data) : data;
    if (parsed.query) {
      animateTextChange(countryName, parsed.country || 'Unknown');
      const code = (parsed.countryCode || '').toLowerCase();
      let flagSrc = null;
      if (code && code !== 'unknown') {
        flagSrc = `https://flagcdn.com/w40/${code}.png`;
      }
      animateIpAndFlag(parsed.query || '---.---.---.---', flagSrc);
      baseIp = parsed.query;
      updateSmartDash('online');
    }
  }).catch(() => {});
}

function updateConnectBtnText() {
  if (!selectedServerLink) return;
  const btnConnect = document.getElementById('btn-connect');
  if (!btnConnect) return;
  const textSpan = btnConnect.querySelector('.connect-text');

  if (isRunning) {
    if (selectedServerLink === connectedServerLink) {
      if (textSpan) textSpan.textContent = 'Disconnect';
      btnConnect.classList.add('disconnect');
      const group = document.getElementById('lcd-capsule');
      if (group) group.classList.add('disconnect');
    } else {
      if (textSpan) textSpan.textContent = 'Switch Server';
      btnConnect.classList.remove('disconnect');
      const group = document.getElementById('lcd-capsule');
      if (group) group.classList.remove('disconnect');
    }
  } else {
    if (textSpan) textSpan.textContent = 'Connect';
    btnConnect.classList.remove('disconnect');
    const group = document.getElementById('lcd-capsule');
    if (group) group.classList.remove('disconnect');
  }
}

// ===== IP Check =====
async function checkIp(useProxy) {
  if (useProxy && !isRunning) return;

  try {
    const data = await invoke('check_ip', { useProxy });
    if (useProxy && !isRunning) return; // Disconnected while awaiting
    
    const parsed = typeof data === 'string' ? JSON.parse(data) : data;

    if (parsed.error) {
      if (isRunning && isConnecting) {
        if (Date.now() - connectStartTime > 2500) {
          log('Connection timeout: Server unreachable.');
          isConnecting = false;
          consecutivePingFailures = 4; // Force ping failure state so it matches
          updateSmartDash('timeout');
          
          if (connectedServerLink) {
            const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
            if (item) {
              item.classList.remove('processing', 'connected');
              item.classList.add('timeout');
            }
          }
          return;
        }
        const pollInterval = (Date.now() - connectStartTime < 3000) ? 250 : 500;
        setTimeout(() => checkIp(useProxy), pollInterval);
        return;
      }
      animateTextChange(countryName, 'Failed');
      animateIpAndFlag('Failed', 'data:image/svg+xml;charset=utf-8,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 24 24%22 fill=%22%2394a3b8%22%3E%3Cpath d=%22M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z%22/%3E%3C/svg%3E');
      return;
    }

    if (isRunning && isConnecting) {
      if (!useProxy && baseIp && parsed.query === baseIp) {
        if (Date.now() - connectStartTime > 15000) {
          log('Connection timeout: Interface routing delay.');
          isConnecting = false;
          consecutivePingFailures = 4;
          updateSmartDash('timeout');
          
          if (connectedServerLink) {
            const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
            if (item) {
              item.classList.remove('processing', 'connected');
              item.classList.add('timeout');
            }
          }
          return;
        }
        const pollInterval2 = (Date.now() - connectStartTime < 3000) ? 250 : 500;
        setTimeout(() => checkIp(useProxy), pollInterval2);
        return;
      }

      if (!isTunRouted) {
        isTunRouted = true;
        runLivePing();
      }
    }

    if (isRunning) {
      if (!useProxy) {
        if (parsed.query && !baseIp) baseIp = parsed.query;
        return; // Prevent delayed local IP check from overwriting VPN IP
      }

      if (isConnecting && isTunRouted) {
        consecutiveReconnects = 0;
        log(`Connected! IP: ${parsed.query}`);
      } else if (!isConnecting && !testingActive) {
        // Do not overwrite ping
      }
    } else {
      // Not connected but got an IP â€” we're online (just idle)
      if (!useProxy) {
        updateSmartDash('online');
        baseIp = parsed.query;
      }
    }

    animateTextChange(countryName, parsed.country || 'Unknown');
    const code = (parsed.countryCode || '').toLowerCase();
    let flagSrc = null;
    if (code && code !== 'unknown') {
      flagSrc = `https://flagcdn.com/w40/${code}.png`;
    }
    animateIpAndFlag(parsed.query || '---.---.---.---', flagSrc);
  } catch (e) {
    if (isRunning && isConnecting) {
      if (Date.now() - connectStartTime > 5000) {
        log('Connection timeout: Network error.');
        isConnecting = false;
        consecutivePingFailures = 4;
        updateSmartDash('timeout');
        
        if (connectedServerLink) {
          const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
          if (item) {
            item.classList.remove('processing', 'connected');
            item.classList.add('timeout');
          }
        }
        return;
      }
      const pollInterval3 = (Date.now() - connectStartTime < 3000) ? 250 : 500;
      setTimeout(() => checkIp(useProxy), pollInterval3);
    }
  }
}

// ===== Live Ping =====
async function runLivePing() {
  if (!isRunning || !connectedServerLink) {
    updateLivePingLabel(null);
    return;
  }

  const currentLink = connectedServerLink;
  const pingStartTime = Date.now();
  try {
    const latency = await invoke('get_live_ping', { link: currentLink });
    if (currentLink === connectedServerLink && connectStartTime <= pingStartTime) {
      updateLivePingLabel(latency);
    }
  } catch (_) {
    if (currentLink === connectedServerLink && connectStartTime <= pingStartTime) {
      updateLivePingLabel(null);
    }
  }
}

function updateLivePingLabel(latency) {
  if (!isRunning) {
    consecutivePingFailures = 0;
    return;
  }

  if (latency === null || latency === 'Timeout' || latency === undefined) {
    consecutivePingFailures++;
    
    if (consecutivePingFailures >= 4) {
      log('Server may be unreachable (ping timeout).');
      if (isConnecting) {
        isConnecting = false; // Give up loading state and show timeout visually
      }
      updateSmartDash('timeout');
      
      if (connectedServerLink) {
        const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
        if (item) {
          item.classList.remove('processing');
          item.classList.remove('connected');
          item.classList.add('timeout');
        }
      }
    } else {
      if (isConnecting) {
        updateSmartDash('processing');
      } else {
        // Suppress transient 'Timeout' error state to prevent UI flashing.
        // It smoothly preserves the last known ping value until the max failure limit is reached.
        updateSmartDash('connected', null);
      }
    }
    return;
  }

  consecutivePingFailures = 0;

  const ms = typeof latency === 'number' ? latency : parseInt(latency);
  
  if (connectedServerLink && (!isConnecting || isTunRouted)) {
    const item = document.querySelector(`.server-item[data-link="${connectedServerLink}"]`);
    if (item) {
      item.classList.remove('processing');
      item.classList.remove('timeout');
      item.classList.add('connected');
    }
  }
  
  if (isConnecting) {
    if (isTunRouted) {
      isConnecting = false;
      updateSmartDash('connected', ms);
    } else {
      updateSmartDash('processing');
    }
  } else {
    updateSmartDash('connected', ms);
  }
}

// ===== Crash Monitor =====
let isReconnecting = false;
let consecutiveReconnects = 0;

async function checkCoreHealth() {
  if (!isRunning || isReconnecting || isConnectToLinkActive) return;

  try {
    const alive = await invoke('is_core_alive');
      if (!alive) {
        // Save the link before any state changes
        const savedLink = connectedServerLink;
        if (!savedLink) return;
        
        isReconnecting = true;
        consecutiveReconnects++;
        updateSmartDash('processing');
        
        if (consecutiveReconnects > 3) {
          log(`Core failed to start ${consecutiveReconnects} times. Giving up.`);
          isReconnecting = false;
          disconnect();
          return;
        }

        log(`Core stopped. Auto-reconnecting (${consecutiveReconnects}) to ${savedLink.substring(0, 40)}...`);
        
        // Wait a moment then reconnect
        setTimeout(async () => {
          if (isConnectToLinkActive || !isReconnecting) {
              log('User initiated connection or cancelled. Aborting auto-reconnect.');
              isReconnecting = false;
              return;
          }
          try {
            connectedServerLink = null; // Reset so connectToLink works clean
            isRunning = false;
            isConnecting = false;
            selectedServerLink = savedLink;
            await connectToLink(savedLink, true);
            isReconnecting = false;
            // We do not reset consecutiveReconnects here; we wait for a successful connection to do it.
          } catch (e) {
            log('Auto-reconnect failed: ' + e + '. Retrying in 3s...');
            // Restore state for next retry attempt
            connectedServerLink = savedLink;
            isRunning = true; // Keep health check alive
            isReconnecting = false;
            // The next health check (every 2s) will detect core is dead and retry
          }
        }, 2000);
      }
  } catch (_) {}
}

// ===== Latency Testing =====
let shouldStopTesting = false;

async function testAllLatency() {
  if (testingActive) {
    shouldStopTesting = true;
    invoke('cancel_latency_tests').catch(()=>{});
    if (btnTest) { btnTest.textContent = 'Stopping...'; btnTest.disabled = true; }
    return;
  }
  
  testingActive = true;
  shouldStopTesting = false;
  invoke('start_latency_tests').catch(()=>{});
  if (btnTest) btnTest.textContent = 'Stop Testing';

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

  const maxConcurrency = 5;
  let currentIndex = 0;
  
  async function worker() {
    while (currentIndex < links.length && !shouldStopTesting) {
      const index = currentIndex++;
      const link = links[index];
      
      updateLatencyUi(link, 'Testing...');
      try {
        const result = await invoke('test_single_latency', { link });
        if (shouldStopTesting) { updateLatencyUi(link, ''); return; }
        const parsed = typeof result === 'string' ? JSON.parse(result) : result;
        const latency = parsed.latency !== undefined ? parsed.latency : parsed;
        updateLatencyUi(link, latency);
      } catch (e) {
        if (!shouldStopTesting) updateLatencyUi(link, 'Error');
      }
    }
  }

  const workers = [];
  for (let i = 0; i < maxConcurrency; i++) {
    workers.push(worker());
  }

  await Promise.all(workers);

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

async function pingSingleItem(link) {
  // Allow concurrent single-server pings (don't block on testingActive)
  // Only block if a full testAllLatency is in progress
  if (testingActive) return;
  updateLatencyUi(link, '...');

  try {
    const result = await invoke('test_single_latency', { link });
    const parsed = typeof result === 'string' ? JSON.parse(result) : result;
    const latency = parsed.latency !== undefined ? parsed.latency : parsed;
    updateLatencyUi(link, latency);
  } catch (e) {
    updateLatencyUi(link, 'Error');
  }
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
  const allBtn = document.getElementById('btn-update-all-subs');
  if (btn) { btn.disabled = true; btn.textContent = 'Updating...'; }
  if (allBtn) {
    allBtn.disabled = true;
    const svg = allBtn.querySelector('.update-all-svg');
    if (svg) svg.classList.add('spin');
  }

  try {
    const subsRaw = await invoke('get_subscriptions');
    const subs = typeof subsRaw === 'string' ? JSON.parse(subsRaw) : subsRaw;

    if (Array.isArray(subs)) {
      for (const sub of subs) {
        const u = typeof sub === 'string' ? sub : (sub.url || sub);
        if (u && u !== 'manual') {
          try {
            await invoke('update_subscription', { url: u });
          } catch (e) {
            console.error('Failed to update ' + u + ': ' + e);
          }
        }
      }
    }
    await refreshServerList();
    await refreshSubscriptions();
    log('Subscriptions updated.');
  } catch (e) {
    log('Update failed: ' + e);
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Update All Subscriptions';
    }
    if (allBtn) {
      allBtn.disabled = false;
      const svg = allBtn.querySelector('.update-all-svg');
      if (svg) svg.classList.remove('spin');
    }
  }
}

async function deleteServer(groupKey, link) {
  // Find server element and trigger Telegram-style deletion animation
  const itemEl = Array.from(document.querySelectorAll('.server-item')).find(el => el.dataset.link === link);
  if (itemEl) {
    itemEl.classList.add('deleting-telegram');
  }

  // Run backend delete in parallel
  const deletePromise = invoke('delete_server', { group: groupKey, link });

  // Wait for the collapse and dissolve animation to complete
  await new Promise(r => setTimeout(r, 340));

  try {
    await deletePromise;
    if (selectedServerLink === link) {
      selectedServerLink = null;
    }
    await refreshServerList();
  } catch (e) {
    log('Delete failed: ' + e);
    await refreshServerList();
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
  shouldStopTesting = false;
  invoke('start_latency_tests').catch(()=>{});

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
  links.forEach(link => updateLatencyUi(link, 'Testing...'));
    try {
      const results = await invoke('test_all_latency', { links });
      const parsed = typeof results === 'string' ? JSON.parse(results) : results;
      
      for (const [link, latency] of Object.entries(parsed)) {
        updateLatencyUi(link, latency);
      }
    } catch (e) {
    if (!shouldStopTesting) {
      links.forEach(link => updateLatencyUi(link, 'Error'));
    }
  }

  // Clean up stopped items
  if (shouldStopTesting) {
    items.forEach(item => {
      const latEl = item.querySelector('.server-latency');
      if (latEl && (latEl.textContent === '...' || latEl.textContent === 'Testing...')) {
        updateLatencyDisplay(latEl, '');
      }
    });
  }

  testingActive = false;
  shouldStopTesting = false;
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
let isTunSwitching = false;

async function handleTunToggle() {
  const checked = tunToggle.checked;
  // Instantly update the visual micro-checkbox so the UI feels smooth and responsive
  updateTunUIState();

  if (checked) {
    try {
      const admin = await invoke('is_admin');
      if (!admin) {
        await invoke('toggle_tun', { enabled: true });
        
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
    updateSmartDash('processing');
    try {
      const linkToReconnect = connectedServerLink;
      
      isConnecting = true;
      isTunRouted = false;
      connectStartTime = Date.now();
      consecutivePingFailures = 0;
      
      try {
        await invoke('switch_tun_mode', { tunMode: checked, link: linkToReconnect });
        
        if (!isRunning || connectedServerLink !== linkToReconnect) {
            log('TUN switch aborted by user.');
            return;
        }

        setTimeout(() => checkIp(true), 200);
        
        // Restart traffic tracking after seamless reconnect
        startTrafficTracking(linkToReconnect);
      } catch (e) {
        log(`Connection failed during TUN switch: ${e}`);
        disconnect();
      } finally {
        isConnectToLinkActive = false;
      }
    } catch (e) {
      log(`TUN mode switch failed: ${e}`);
      if (consecutivePingFailures >= 4) {
        updateSmartDash('timeout');
      } else {
        updateSmartDash('connected'); // Revert to connected on failure
      }
    }
  }
}

// ===== Logs =====
function toggleLogs() {
  logsOpen = !logsOpen;
  logViewer.classList.toggle('open', logsOpen);
  if (btnLogs) btnLogs.classList.toggle('active', logsOpen);
}

function forceShowLogs() {
  logsOpen = true;
  logViewer.classList.add('open');
  if (btnLogs) btnLogs.classList.add('active');
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
    await invoke('copy_to_clipboard', { text });
  } catch (err) {
    console.warn('Native clipboard copy failed, falling back:', err);
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

// Disable default right-click across the entire application.
// Only right-clicking on a subscription header opens its 3-dots dropdown menu.
window.addEventListener('contextmenu', (e) => {
  e.preventDefault();

  const groupHeader = e.target.closest('.group-header');
  if (groupHeader && !e.target.closest('.group-dropdown-menu')) {
    e.stopPropagation();
    const dropdownMenu = groupHeader.querySelector('.group-dropdown-menu');
    if (dropdownMenu) {
      document.querySelectorAll('.group-dropdown-menu.show').forEach(el => {
        if (el !== dropdownMenu) el.classList.remove('show');
      });
      dropdownMenu.classList.toggle('show');
    }
  }
});

// ===== Selected Server Ping Shortcut =====
function triggerPingTestShortcut() {
  // Check if a server is selected (has white glowing plate / selectedServerLink)
  let targetLink = selectedServerLink;
  if (!targetLink) {
    const selectedItem = document.querySelector('.server-item.selected');
    if (selectedItem && selectedItem.dataset.link) {
      targetLink = selectedItem.dataset.link;
    }
  }

  if (targetLink) {
    // Only test the single server that was clicked/selected
    pingSingleItem(targetLink);
  } else {
    log('Please click on a server to select it first.');
  }
}

// ===== Server Copied Visual Feedback =====
function showServerCopiedFeedback(itemOrLink) {
  let el = null;
  if (typeof itemOrLink === 'string') {
    el = Array.from(document.querySelectorAll('.server-item')).find(item => item.dataset.link === itemOrLink);
  } else if (itemOrLink instanceof HTMLElement) {
    el = itemOrLink.closest('.server-item') || itemOrLink;
  }
  if (!el) {
    el = document.querySelector('.server-item.selected');
  }
  if (!el) return;

  const nameEl = el.querySelector('.server-name');
  if (nameEl) {
    if (nameEl.dataset.isCopying === 'true') return;
    nameEl.dataset.isCopying = 'true';
    const origHtml = nameEl.innerHTML;
    const origText = nameEl.textContent;

    // Pure glowing animated typography (exact text "Copied Link", no capsule, no box, no checkmark)
    nameEl.innerHTML = '<span class="copied-link-text">Copied Link</span>';

    setTimeout(() => {
      const textSpan = nameEl.querySelector('.copied-link-text');
      if (textSpan) {
        textSpan.classList.add('fade-out');
        setTimeout(() => {
          if (nameEl && nameEl.dataset.isCopying === 'true') {
            nameEl.innerHTML = origHtml || origText;
            nameEl.style.animation = 'fancyTextIn 0.2s cubic-bezier(0.16, 1, 0.3, 1) forwards';
            setTimeout(() => {
              if (nameEl) nameEl.style.animation = '';
              delete nameEl.dataset.isCopying;
            }, 210);
          }
        }, 190);
      } else {
        nameEl.innerHTML = origHtml || origText;
        delete nameEl.dataset.isCopying;
      }
    }, 1100);
  }
}

// ===== Selected Server Copy Shortcut =====
async function triggerCopyShortcut() {
  let targetLink = selectedServerLink;
  let targetItem = null;
  if (!targetLink) {
    targetItem = document.querySelector('.server-item.selected');
    if (targetItem && targetItem.dataset.link) {
      targetLink = targetItem.dataset.link;
    }
  } else {
    targetItem = Array.from(document.querySelectorAll('.server-item')).find(el => el.dataset.link === targetLink);
  }

  if (targetLink) {
    await copyToClipboard(targetLink);
    showServerCopiedFeedback(targetItem || targetLink);
    log('Server link copied to clipboard.');
  } else {
    log('Please click on a server to select it first.');
  }
}

// ===== Global Keyboard Shortcuts =====
window.addEventListener('keydown', async (e) => {
  const isCtrlOrCmd = e.ctrlKey || e.metaKey;
  const isKeyR = e.code === 'KeyR' || e.keyCode === 82 || e.which === 82 || (e.key && e.key.toLowerCase() === 'r') || e.key === 'Ù‚';
  const isKeyV = e.code === 'KeyV' || e.keyCode === 86 || e.which === 86 || (e.key && e.key.toLowerCase() === 'v') || e.key === 'Ø±';
  const isKeyC = e.code === 'KeyC' || e.keyCode === 67 || e.which === 67 || (e.key && e.key.toLowerCase() === 'c') || e.key === 'Ú˜' || e.key === 'Ø²';
  const isF5 = e.key === 'F5' || e.keyCode === 116 || e.code === 'F5';

  // 1. Enter in subscription add input
  if (e.key === 'Enter' && document.activeElement?.id === 'txt-input') {
    handleSmartAdd();
    return;
  }

  // 2. Escape to close any open modal
  if (e.key === 'Escape' || e.keyCode === 27) {
    closeImportModal();
    closeEditSubModal();
    closeEditServerModal();
    closeStatsModal();
    return;
  }

  // 3. Ctrl+R / Cmd+R / F5: Test real delay ping of ONLY the selected server (disable browser reload)
  if ((isCtrlOrCmd && isKeyR) || isF5) {
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();
    triggerPingTestShortcut();
    return;
  }

  // 4. Ctrl+C / Cmd+C: Copy link of the selected server
  if (isCtrlOrCmd && isKeyC) {
    const selection = window.getSelection()?.toString();
    const active = document.activeElement;
    const isInput = active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA' || active.isContentEditable);

    // If user has highlighted text or is selecting text inside an input, preserve normal copy
    if ((selection && selection.trim().length > 0) || (isInput && active.selectionStart !== active.selectionEnd)) {
      return;
    }

    let targetLink = selectedServerLink || document.querySelector('.server-item.selected')?.dataset.link;
    if (targetLink) {
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();
      await triggerCopyShortcut();
    }
    return;
  }

  // 5. Ctrl+V / Cmd+V: Global paste to import servers or subscriptions
  if (isCtrlOrCmd && isKeyV) {
    const active = document.activeElement;
    const isInput = active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA' || active.isContentEditable);
    
    // If typing in another input (e.g. search, renaming), let browser handle normal paste
    if (isInput && active.id !== 'txt-import-content') {
      return;
    }

    // If already inside the import textarea, let normal paste happen and update preview
    if (active && active.id === 'txt-import-content') {
      setTimeout(updateImportPreview, 20);
      return;
    }

    // Global paste triggered from outside inputs
    e.preventDefault();
    e.stopPropagation();
    await handleGlobalPasteShortcut();
  }
}, { capture: true, passive: false });

async function handleGlobalPasteShortcut() {
  try {
    let text = '';
    try {
      text = await invoke('read_from_clipboard');
    } catch (_) {
      text = await navigator.clipboard.readText();
    }
    if (!text || !text.trim()) {
      openImportModal();
      return;
    }

    const { servers, subscriptions } = parseImportContent(text);
    openImportModal();
    const textarea = document.getElementById('txt-import-content');
    if (textarea) {
      textarea.value = text.trim();
      updateImportPreview();
    }

    if (servers.length > 0 || subscriptions.length > 0) {
      await executeBatchImport();
    }
  } catch (err) {
    console.warn('Clipboard read error on Ctrl+V:', err);
    openImportModal();
  }
}


function escapeHtml(str) {
  return String(str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

async function refreshSubscriptions() {
  try {
    const subsRaw = await invoke('get_subscriptions');
    const subs = typeof subsRaw === 'string' ? JSON.parse(subsRaw) : subsRaw;
    const listEl = document.getElementById('subscriptions-list');
    if (!listEl) return;
    listEl.innerHTML = '';

    if (!subs || !Array.isArray(subs) || subs.length === 0) {
      listEl.innerHTML = `
        <div style="text-align: center; padding: 48px 20px; color: var(--text-secondary); background: rgba(255,255,255,0.02); border: 1px dashed rgba(255,255,255,0.08); border-radius: 14px;">
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="opacity: 0.4; margin-bottom: 10px;"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>
          <div style="font-size: 15px; font-weight: 600; color: #ffffff; margin-bottom: 4px;">No Subscriptions Added</div>
          <div style="font-size: 12px; color: #888888;">Paste a subscription URL above and click Add to start managing your proxies.</div>
        </div>
      `;
      return;
    }

    subs.forEach(sub => {
      const url = sub.url;
      const hash = hashCode(url);
      const card = document.createElement('div');
      card.className = 'sub-manage-card';
      card.dataset.url = url;

      // Quota calculation
      let quotaHtml = '';
      let remainingQuotaText = '-';
      if (sub.total && sub.total > 0) {
        const used = (sub.upload || 0) + (sub.download || 0);
        const rem = Math.max(0, sub.total - used);
        remainingQuotaText = formatBytes(rem);
        const pct = Math.min(100, Math.max(0, Math.round((used / sub.total) * 100)));
        const barClass = pct >= 90 ? 'critical' : (pct >= 70 ? 'warning' : 'success');
        
        let expireText = '';
        if (sub.expire) {
          const now = Math.floor(Date.now() / 1000);
          const days = Math.max(0, Math.floor((sub.expire - now) / 86400));
          expireText = `<span class="sub-quota-expire">${days} days left</span>`;
        }

        quotaHtml = `
          <div class="sub-quota-box">
            <div class="sub-quota-header">
              <span class="sub-quota-used">${formatBytes(used)} / ${formatBytes(sub.total)} (${pct}%)</span>
              ${expireText}
            </div>
            <div class="sub-quota-bar-wrapper">
              <div class="sub-quota-bar-fill ${barClass}" style="width: ${pct}%;"></div>
            </div>
          </div>
        `;
      } else if (sub.expire) {
        const now = Math.floor(Date.now() / 1000);
        const days = Math.max(0, Math.floor((sub.expire - now) / 86400));
        quotaHtml = `
          <div class="sub-quota-box">
            <div class="sub-quota-header">
              <span class="sub-quota-used">Unlimited Quota</span>
              <span class="sub-quota-expire">${days} days left</span>
            </div>
          </div>
        `;
      }

      // Latency calculation
      const pings = sub.stats?.ping_history || [];
      const avgPing = pings.length > 0 ? Math.round(pings.reduce((a, b) => a + b, 0) / pings.length) + ' ms' : '- ms';

      const serializedSub = JSON.stringify(sub).replace(/"/g, '&quot;');

      card.innerHTML = `
        <div class="sub-card-header">
          <div class="sub-card-title-group">
            <h3 class="sub-card-title" title="${escapeHtml(sub.name || url)}">${escapeHtml(sub.name || url)}</h3>
            <div class="sub-badges">
              <span class="sub-badge servers-badge">${sub.servers_count || 0} Servers</span>
              ${sub.auto_update ? `<span class="sub-badge update-badge">Auto: ${sub.update_interval || 24}h</span>` : `<span class="sub-badge off-badge">Manual</span>`}
              ${sub.pinned ? `<span class="sub-badge pin-badge">Pinned</span>` : ''}
            </div>
          </div>
          <div class="sub-card-actions">
            <button class="sub-act-btn btn-update" id="sub-update-btn-${hash}" title="Update Now" onclick="updateSingleSubscriptionCard('${url}', this)">
              <svg class="sub-act-svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><polyline points="1 20 1 14 7 14"></polyline><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg>
            </button>
            <button class="sub-act-btn btn-stats-toggle" id="sub-stats-toggle-${hash}" title="Analytics & Daily Usage" onclick="toggleCardStats('${url}', this)">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 20V10M12 20V4M6 20v-6"></path></svg>
            </button>
            <button class="sub-act-btn btn-edit" title="Edit Subscription" onclick='openEditSubModal("${url}", ${serializedSub})'>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
            </button>
            <button class="sub-act-btn btn-delete" title="Delete Subscription" onclick="deleteSubscriptionFromTab('${url}')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
            </button>
          </div>
        </div>

        <div class="sub-card-body">
          <div class="sub-url-row" onclick="copyToClipboard('${url}')" title="Click to copy subscription URL">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>
            <span class="sub-url-text">${url}</span>
            <span class="sub-copy-hint">Copy</span>
          </div>
          ${quotaHtml}
          ${sub.announcement ? `
            <div class="sub-announcement-strip">
              <span class="announcement-badge-pill">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M11 5L6 9H2v6h4l5 4V5z"></path>
                  <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
                  <path d="M19.07 4.93a10 10 0 0 1 0 14.14"></path>
                </svg>
                <span class="announcement-badge-label">ANNOUNCEMENT</span>
              </span>
              <span style="flex: 1; color: #ffffff;">${escapeHtml(sub.announcement)}</span>
              <span class="announcement-live-dot"></span>
            </div>
          ` : ''}
        </div>

        <!-- Embedded Live Analytics Drawer -->
        <div class="sub-card-stats-drawer" id="stats-drawer-${hash}" style="display: none;">
          <div class="sub-stats-metrics">
            <div class="sub-metric-card">
              <span class="sub-metric-label">Client Usage</span>
              <span class="sub-metric-value" id="client-usage-${hash}">${formatBytes(sub.stats?.total_used || 0)}</span>
            </div>
            <div class="sub-metric-card">
              <span class="sub-metric-label">Avg Ping</span>
              <span class="sub-metric-value">${avgPing}</span>
            </div>
            <div class="sub-metric-card">
              <span class="sub-metric-label">Remaining Data</span>
              <span class="sub-metric-value">${remainingQuotaText}</span>
            </div>
            <div class="sub-metric-card">
              <span class="sub-metric-label">Auto Update</span>
              <span class="sub-metric-value">${sub.auto_update ? sub.update_interval + 'h' : 'Manual'}</span>
            </div>
          </div>
          <div class="sub-stats-chart-wrapper">
            <canvas id="sub-chart-${hash}" class="sub-canvas" width="550" height="130"></canvas>
          </div>
        </div>
      `;

      listEl.appendChild(card);
    });
  } catch (e) {
    console.error("Failed to load subs:", e);
  }
}

async function updateSingleSubscriptionCard(url, btn) {
  const svg = btn ? btn.querySelector('.sub-act-svg') : null;
  if (svg) svg.classList.add('spin');
  try {
    await invoke('update_subscription', { url });
    await refreshSubscriptions();
    await refreshServerList();
  } catch (e) {
    console.error('Update failed:', e);
  } finally {
    if (svg) svg.classList.remove('spin');
  }
}

async function deleteSubscriptionFromTab(url) {
  try {
    await invoke('delete_subscription', { groupKey: url });
    await refreshSubscriptions();
    await refreshServerList();
  } catch (e) {
    console.error('Delete failed:', e);
  }
}

async function toggleCardStats(url, btn) {
  const hash = hashCode(url);
  const drawer = document.getElementById(`stats-drawer-${hash}`);
  if (!drawer) return;
  const isHidden = drawer.style.display === 'none';
  drawer.style.display = isHidden ? 'flex' : 'none';
  if (btn) btn.classList.toggle('active', isHidden);

  if (isHidden) {
    try {
      const stats = await invoke('get_item_stats', { isGroup: true, key: url });
      const canvas = document.getElementById(`sub-chart-${hash}`);
      if (canvas && stats) {
        setTimeout(() => renderDailyUsageGraph(canvas, stats.daily_usage || {}), 50);
      }
      const usageEl = document.getElementById(`client-usage-${hash}`);
      if (usageEl && stats) {
        usageEl.textContent = formatBytes(stats.total_used || 0);
      }
    } catch (e) {
      console.error(e);
    }
  }
}

function selectIntervalChip(hours) {
  const input = document.getElementById('edit-sub-interval');
  if (input) input.value = hours;
  document.querySelectorAll('.interval-chip').forEach(chip => {
    chip.classList.toggle('active', parseInt(chip.getAttribute('data-hours')) === hours);
  });
}

function copyCurrentEditUrl() {
  if (currentEditSubKey) {
    copyToClipboard(currentEditSubKey);
    const btn = document.getElementById('btn-edit-copy-url');
    if (btn) {
      const span = btn.querySelector('span');
      if (span) {
        const orig = span.textContent;
        span.textContent = 'Copied!';
        setTimeout(() => span.textContent = orig, 1500);
      }
    }
  }
}

function cleanRoutingPattern(raw) {
    let p = raw.trim();
    // Strip protocol
    p = p.replace(/^https?:\/\//i, '');
    // Strip trailing slashes and paths
    p = p.replace(/\/.*$/, '');
    // Strip port
    p = p.replace(/:\d+$/, '');
    // Strip leading/trailing dots
    p = p.replace(/^\.+|\.+$/g, '');
    return p;
}

function handleAddRoutingRule() {
    const rawPattern = document.getElementById('txt-rule-pattern').value;
    const action = document.getElementById('select-rule-action').value;
    if (!rawPattern.trim()) return;
    const pattern = cleanRoutingPattern(rawPattern);
    if (!pattern) return;
    const rule = {
      id: 'rule_' + Date.now() + '_' + Math.random().toString(36).substr(2, 5),
      pattern: pattern,
      action: action,
      enabled: true,
    };
    invoke('add_routing_rule', { rule: rule }).then(() => {
        document.getElementById('txt-rule-pattern').value = '';
        refreshRoutingRules();
        if(window.restartConnectionIfActive) window.restartConnectionIfActive();
    }).catch(e => console.error(e));
}

document.addEventListener("DOMContentLoaded", () => {
    refreshSubscriptions();
});

function switchSettingsTab(tabId) {
  document.querySelectorAll('.settings-tab-btn').forEach(btn => {
    btn.classList.remove('active');
  });
  document.querySelectorAll('#page-settings .settings-pane').forEach(pane => {
    pane.classList.remove('active');
  });
  
  // Find the clicked button by matching its text content
  document.querySelectorAll('.settings-tab-btn').forEach(btn => {
    if (btn.textContent.toLowerCase() === tabId) {
      btn.classList.add('active');
    }
  });
  const pane = document.getElementById('settings-' + tabId);
  if (pane) pane.classList.add('active');
}

// ===== Routing Preset Listeners =====
function initRoutingPresets() {
  const iranToggle = document.getElementById('toggle-preset-iran');
  const lanToggle = document.getElementById('toggle-preset-lan');
  const adsToggle = document.getElementById('toggle-preset-ads');
  
  if (iranToggle) iranToggle.addEventListener('change', () => {
    invoke('update_routing_preset', { preset: 'bypass_iran', enabled: iranToggle.checked })
      .then(() => { if(window.restartConnectionIfActive) window.restartConnectionIfActive(); })
      .catch(e => console.error(e));
  });
  if (lanToggle) lanToggle.addEventListener('change', () => {
    invoke('update_routing_preset', { preset: 'bypass_lan', enabled: lanToggle.checked })
      .then(() => { if(window.restartConnectionIfActive) window.restartConnectionIfActive(); })
      .catch(e => console.error(e));
  });
  if (adsToggle) adsToggle.addEventListener('change', () => {
    invoke('update_routing_preset', { preset: 'block_ads', enabled: adsToggle.checked })
      .then(() => { if(window.restartConnectionIfActive) window.restartConnectionIfActive(); })
      .catch(e => console.error(e));
  });

  // Load initial preset states
  invoke('get_routing_settings').then(data => {
    if (iranToggle) iranToggle.checked = !!data.bypass_iran;
    if (lanToggle) lanToggle.checked = !!data.bypass_lan;
    if (adsToggle) adsToggle.checked = !!data.block_ads;
  }).catch(e => console.error('Failed to load routing presets:', e));
}

// ===== Settings Toggle Listeners =====
function initSettingsListeners() {

  const toggleBlurIp = document.getElementById('toggle-blur-ip');
  const ipAddress = document.getElementById('ip-address');
  if (toggleBlurIp && ipAddress) {
    const isBlur = localStorage.getItem('blur_ip') === 'true';
    toggleBlurIp.checked = isBlur;
    if (isBlur) ipAddress.classList.add('spoiler');
    toggleBlurIp.addEventListener('change', (e) => {
      localStorage.setItem('blur_ip', e.target.checked);
      if (e.target.checked) ipAddress.classList.add('spoiler');
      else ipAddress.classList.remove('spoiler');
    });
  }

  const autoStart = document.getElementById('toggle-auto-start');
  const startMin = document.getElementById('toggle-start-minimized');
  const autoUpdate = document.getElementById('toggle-auto-update-subs');
  const autoReconnect = document.getElementById('toggle-auto-reconnect');
  const livePing = document.getElementById('toggle-live-ping');
  const tlsFragment = document.getElementById('toggle-tls-fragment');
  const remoteDns = document.getElementById('input-remote-dns');
  const localDns = document.getElementById('input-local-dns');
  const coreSelect = document.getElementById('select-core');
  
  if (autoStart) autoStart.addEventListener('change', () => {
    invoke('set_auto_start', { enabled: autoStart.checked }).catch(e => console.error(e));
    // Enable/disable start minimized when auto start changes
    const minRow = document.getElementById('toggle-start-minimized');
    if (minRow) minRow.closest('.setting-item').style.opacity = autoStart.checked ? '1' : '0.4';
    if (minRow) minRow.closest('.setting-item').style.pointerEvents = autoStart.checked ? 'auto' : 'none';
  });
  if (startMin) startMin.addEventListener('change', () => {
    invoke('set_start_minimized', { enabled: startMin.checked }).catch(e => console.error(e));
  });
  if (autoUpdate) autoUpdate.addEventListener('change', () => {
    invoke('set_auto_update_subs', { enabled: autoUpdate.checked }).catch(e => console.error(e));
  });
  if (tlsFragment) tlsFragment.addEventListener('change', updateCoreSettings);
  
  const strictRoute = document.getElementById('toggle-strict-route');
  if (strictRoute) strictRoute.addEventListener('change', updateCoreSettings);

  // Custom Dropdown Logic
  const coreDropdown = document.getElementById('core-dropdown');
  const coreDropdownSelected = document.getElementById('core-dropdown-selected');
  const coreDropdownOptions = document.querySelectorAll('#core-dropdown .custom-dropdown-option');
  
  if (coreDropdown && coreDropdownSelected && coreSelect) {
    // Toggle dropdown
    coreDropdownSelected.addEventListener('click', (e) => {
      e.stopPropagation();
      coreDropdown.classList.toggle('open');
    });

    // Close when clicking outside
    document.addEventListener('click', () => {
      coreDropdown.classList.remove('open');
    });

    // Handle option click
    coreDropdownOptions.forEach(opt => {
      opt.addEventListener('click', (e) => {
        e.stopPropagation();
        const val = opt.getAttribute('data-value');
        
        // Update UI
        coreDropdownSelected.textContent = opt.textContent.trim();
        coreDropdownOptions.forEach(o => o.classList.remove('active'));
        opt.classList.add('active');
        
        // Update hidden input and trigger change
        coreSelect.value = val;
        coreDropdown.classList.remove('open');
        updateCoreSettings();
      });
    });
  }

  // Custom Dropdown Logic for Action
  const actionDropdown = document.getElementById('action-dropdown');
  const actionDropdownSelected = document.getElementById('action-dropdown-selected');
  const actionDropdownOptions = document.querySelectorAll('#action-dropdown .custom-dropdown-option');
  const actionSelect = document.getElementById('select-rule-action');

  if (actionDropdown && actionDropdownSelected && actionSelect) {
    actionDropdownSelected.addEventListener('click', (e) => {
      e.stopPropagation();
      actionDropdown.classList.toggle('open');
    });

    document.addEventListener('click', () => {
      actionDropdown.classList.remove('open');
    });

    actionDropdownOptions.forEach(opt => {
      opt.addEventListener('click', (e) => {
        e.stopPropagation();
        const val = opt.getAttribute('data-value');
        
        actionDropdownSelected.innerHTML = `<span class="dropdown-value">${opt.innerHTML}</span>`;
        actionDropdownOptions.forEach(o => o.classList.remove('active'));
        opt.classList.add('active');
        
        actionSelect.value = val;
        actionDropdown.classList.remove('open');
      });
    });
  }

  // Fallback native listener just in case
  if (coreSelect) coreSelect.addEventListener('change', updateCoreSettings);

  let restartDebounceTimer = null;
  async function restartConnectionIfActive() {
    if (restartDebounceTimer) clearTimeout(restartDebounceTimer);
    restartDebounceTimer = setTimeout(async () => {
        if (isRunning && connectedServerLink) {
          console.log('Routing/Core settings changed. Reconnecting automatically...');
          const link = connectedServerLink;
          connectedServerLink = null; // Clean state for a fresh connectToLink
          isRunning = false;
          isConnecting = false;
          await connectToLink(link, true);
        }
    }, 800);
  }

  // Export for global access by inline onclicks
  window.restartConnectionIfActive = restartConnectionIfActive;

  function updateCoreSettings() {
    const frag = tlsFragment ? tlsFragment.checked : false;
    const strict = strictRoute ? strictRoute.checked : false;
    const core = coreSelect ? coreSelect.value : 'auto';
    invoke('update_core_settings', { fragment: frag, strictRoute: strict, coreChoice: core })
      .then(() => restartConnectionIfActive())
      .catch(e => console.error(e));
  }
  if (autoReconnect) autoReconnect.addEventListener('change', () => {
    invoke('set_auto_reconnect', { enabled: autoReconnect.checked }).catch(e => console.error(e));
  });
    if (livePing) livePing.addEventListener('change', () => {
    invoke('set_live_ping_enabled', { enabled: livePing.checked }).catch(e => console.error(e));
    const wrapper = document.querySelector('.micro-wrapper');
    if (wrapper) {
      if (livePing.checked) {
        wrapper.classList.remove('ping-off');
      } else {
        wrapper.classList.add('ping-off');
      }
    }
  });
  // Duplicate listeners removed â€” updateCoreSettings already handles these via lines 1716/1721
  if (remoteDns) remoteDns.addEventListener('change', () => {
    const local = localDns ? localDns.value : 'local';
    invoke('update_dns_settings', { remote: remoteDns.value, local: local }).catch(e => console.error(e));
  });
  if (localDns) localDns.addEventListener('change', () => {
    const remote = remoteDns ? remoteDns.value : 'https://1.1.1.1/dns-query';
    invoke('update_dns_settings', { remote: remote, local: localDns.value }).catch(e => console.error(e));
  });
}

// ===== Load Settings Values from Backend =====
async function loadSettingsValues() {
  try {
    const settings = await invoke('get_startup_settings');
    if (!settings) return;
    
    const setChecked = (id, val) => { const el = document.getElementById(id); if (el) el.checked = !!val; };
    setChecked('toggle-auto-start', settings.auto_start);
    setChecked('toggle-start-minimized', settings.start_minimized);
    setChecked('toggle-auto-update-subs', settings.auto_update_subs);
    setChecked('toggle-auto-reconnect', settings.auto_reconnect);
        setChecked('toggle-live-ping', settings.live_ping_enabled);
    const wrapper = document.querySelector('.micro-wrapper');
    if (wrapper) {
      if (settings.live_ping_enabled) {
        wrapper.classList.remove('ping-off');
      } else {
        wrapper.classList.add('ping-off');
      }
    }
    
    // Disable start minimized if auto start is off
    const minItem = document.getElementById('toggle-start-minimized');
    if (minItem && !settings.auto_start) {
      minItem.closest('.setting-item').style.opacity = '0.4';
      minItem.closest('.setting-item').style.pointerEvents = 'none';
    }
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
  
  // Load core settings (core_choice, tls_fragment, strict_route)
  try {
    const coreSettings = await invoke('get_core_settings');
    if (coreSettings) {
      const coreEl = document.getElementById('select-core');
      if (coreEl) {
        coreEl.value = coreSettings.core_choice || 'auto';
        
        // Update custom dropdown UI
        const coreDropdownSelected = document.getElementById('core-dropdown-selected');
        const options = document.querySelectorAll('#core-dropdown .custom-dropdown-option');
        options.forEach(opt => {
          if (opt.getAttribute('data-value') === coreEl.value) {
            opt.classList.add('active');
            if (coreDropdownSelected) coreDropdownSelected.textContent = opt.textContent.trim();
          } else {
            opt.classList.remove('active');
          }
        });
      }
      
      const fragEl = document.getElementById('toggle-tls-fragment');
      if (fragEl) fragEl.checked = !!coreSettings.tls_fragment;
      const strictEl = document.getElementById('toggle-strict-route');
      if (strictEl) strictEl.checked = !!coreSettings.strict_route;
    }
  } catch (_) {}
}

// ===== Statistics Modal =====
let statsModalInterval = null;
let currentStatsKey = null;
let currentStatsIsGroup = false;
let currentStatsGroupData = null;

function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function openStatsModal(key, isGroup, title, groupData) {
  currentStatsKey = key;
  currentStatsIsGroup = isGroup;
  currentStatsGroupData = groupData;
  const modal = document.getElementById('stats-modal');
  const titleEl = document.getElementById('stats-modal-title');
  if (titleEl) titleEl.textContent = title || 'Statistics';
  
  // Show/hide traffic and expire cards for subscriptions
  const trafficCard = document.getElementById('stats-traffic-card');
  const expireCard = document.getElementById('stats-expire-card');
  if (isGroup && groupData) {
    if (groupData.total && trafficCard) {
      trafficCard.style.display = '';
      const used = (groupData.upload || 0) + (groupData.download || 0);
      const left = groupData.total - used;
      document.getElementById('stats-traffic-left').textContent = formatBytes(Math.max(0, left));
    }
    if (groupData.expire && expireCard) {
      expireCard.style.display = '';
      const now = Math.floor(Date.now() / 1000);
      const daysLeft = Math.max(0, Math.floor((groupData.expire - now) / 86400));
      document.getElementById('stats-days-left').textContent = daysLeft + ' days';
    }
  } else {
    if (trafficCard) trafficCard.style.display = 'none';
    if (expireCard) expireCard.style.display = 'none';
  }
  
  modal.style.display = 'flex';
  refreshStatsData();
  statsModalInterval = setInterval(refreshStatsData, 1000);
}

function closeStatsModal() {
  const modal = document.getElementById('stats-modal');
  modal.style.display = 'none';
  if (statsModalInterval) { clearInterval(statsModalInterval); statsModalInterval = null; }
  currentStatsKey = null;
}

// ===== Edit Server Modal =====
let currentEditServerGroup = null;
let currentEditServerLink = null;
let currentEditServerParsed = null;

function parseVpnLink(link) {
  if (link.startsWith('vmess://')) {
    try {
      let b64 = link.substring(8).replace(/-/g, '+').replace(/_/g, '/');
      while (b64.length % 4) b64 += '=';
      const obj = JSON.parse(decodeURIComponent(escape(window.atob(b64))));
      return { type: 'vmess', remark: obj.ps || '', address: obj.add || '', port: obj.port || '', id: obj.id || '', sni: obj.sni || '', host: obj.host || '', orig: obj };
    } catch(e) {
      console.error('VMESS parse error:', e);
      return null;
    }
  }
  try {
    const idx = link.indexOf('://');
    if (idx === -1) return null;
    
    const type = link.substring(0, idx);
    let rest = link.substring(idx + 3);
    
    let remark = '';
    const hashIdx = rest.indexOf('#');
    if (hashIdx !== -1) {
        remark = decodeURIComponent(rest.substring(hashIdx + 1));
        rest = rest.substring(0, hashIdx);
    }
    
    let query = '';
    const queryIdx = rest.indexOf('?');
    if (queryIdx !== -1) {
        query = rest.substring(queryIdx + 1);
        rest = rest.substring(0, queryIdx);
    }
    
    let id = '';
    const atIdx = rest.indexOf('@');
    if (atIdx !== -1) {
        id = rest.substring(0, atIdx);
        rest = rest.substring(atIdx + 1);
    }
    
    let address = '';
    let port = '';
    const portIdx = rest.lastIndexOf(':');
    if (portIdx !== -1 && !rest.includes(']')) {
        address = rest.substring(0, portIdx);
        port = rest.substring(portIdx + 1);
    } else if (rest.includes(']:')) {
        const split = rest.split(']:');
        address = split[0] + ']';
        port = split[1];
    } else {
        address = rest;
    }
    
    const params = new URLSearchParams(query);
    const sni = params.get('sni') || '';
    const host = params.get('host') || '';
    
    return { type, remark, address, port, id, sni, host, params };
  } catch (e) {
    console.error('VPN parse error:', e);
    return null;
  }
}

function stringifyVpnLink(parsed) {
  if (parsed.type === 'vmess') {
    const obj = { ...parsed.orig, ps: parsed.remark, add: parsed.address, port: String(parsed.port), id: parsed.id, sni: parsed.sni, host: parsed.host };
    return 'vmess://' + window.btoa(unescape(encodeURIComponent(JSON.stringify(obj))));
  }
  if (parsed.sni) parsed.params.set('sni', parsed.sni); else parsed.params.delete('sni');
  if (parsed.host) parsed.params.set('host', parsed.host); else parsed.params.delete('host');
  
  const qs = parsed.params.toString();
  const qStr = qs ? '?' + qs : '';
  const hashStr = parsed.remark ? '#' + encodeURIComponent(parsed.remark) : '';
  const idStr = parsed.id ? parsed.id + '@' : '';
  return `${parsed.type}://${idStr}${parsed.address}${parsed.port ? ':' + parsed.port : ''}${qStr}${hashStr}`;
}

function openEditServerModal(groupKey, link) {
  try {
    currentEditServerGroup = groupKey;
    currentEditServerLink = link;
    
    const parsed = parseVpnLink(link);
    if (!parsed) {
      alert('This link format cannot be edited right now. It might be malformed or an unsupported protocol.');
      return;
    }
    currentEditServerParsed = parsed;
    
    if (!document.getElementById('edit-server-modal')) {
      alert('HTML Modal Missing! Please run npm run tauri dev again.');
      return;
    }
    
    document.getElementById('edit-svr-remark').value = parsed.remark || '';
    document.getElementById('edit-svr-address').value = parsed.address || '';
    document.getElementById('edit-svr-port').value = parsed.port || '';
    document.getElementById('edit-svr-id').value = parsed.id || '';
    document.getElementById('edit-svr-sni').value = parsed.sni || '';
    document.getElementById('edit-svr-host').value = parsed.host || '';
    
    document.getElementById('edit-server-modal').style.display = 'flex';
  } catch (e) {
    alert('Crash: ' + e.message + '\n' + e.stack);
  }
}

function closeEditServerModal() {
  const modal = document.getElementById('edit-server-modal');
  if (modal) modal.style.display = 'none';
}

async function saveServerEdits() {
  if (!currentEditServerParsed) return;
  
  const oldRemark = currentEditServerParsed.remark;
  let newRemark = document.getElementById('edit-svr-remark').value.trim();
  
  const oldFlag = extractFlagEmoji(oldRemark || '');
  const newFlag = extractFlagEmoji(newRemark);
  if (oldFlag && !newFlag) {
    newRemark = oldFlag + ' ' + newRemark;
  }
  
  currentEditServerParsed.remark = newRemark;
  currentEditServerParsed.address = document.getElementById('edit-svr-address').value.trim();
  currentEditServerParsed.port = document.getElementById('edit-svr-port').value.trim();
  currentEditServerParsed.id = document.getElementById('edit-svr-id').value.trim();
  currentEditServerParsed.sni = document.getElementById('edit-svr-sni').value.trim();
  currentEditServerParsed.host = document.getElementById('edit-svr-host').value.trim();
  
  const newLink = stringifyVpnLink(currentEditServerParsed);
  
  if (newLink === currentEditServerLink) {
    closeEditServerModal();
    return;
  }
  
  try {
    await invoke('edit_server', {
      group: currentEditServerGroup,
      oldLink: currentEditServerLink,
      newLink: newLink
    });
    
    // Update connected link if we were connected to it!
    if (connectedServerLink === currentEditServerLink) {
       connectedServerLink = newLink;
    }
    if (selectedServerLink === currentEditServerLink) {
       selectedServerLink = newLink;
    }
    
    log('Server updated successfully.');
    closeEditServerModal();
    await refreshServerList(); // Refresh UI
  } catch (e) {
    log('Failed to edit server: ' + e);
  }
}

// ===== Edit Subscription Modal =====
let currentEditSubKey = null;

function openEditSubModal(groupKey, groupData) {
  currentEditSubKey = groupKey;
  const modal = document.getElementById('edit-sub-modal');
  if (modal) modal.style.display = 'flex';

  const nameInput = document.getElementById('edit-sub-name');
  if (nameInput) nameInput.value = groupData.name || groupKey;
  
  const urlInput = document.getElementById('edit-sub-url');
  if (urlInput) urlInput.value = groupKey;

  const countEl = document.getElementById('edit-sub-server-count');
  if (countEl) {
    const c = (groupData.servers ? groupData.servers.length : groupData.servers_count) || 0;
    countEl.textContent = `${c} Servers`;
  }

  const autoToggle = document.getElementById('edit-sub-auto-update');
  if (autoToggle) autoToggle.checked = groupData.auto_update !== false;

  const interval = groupData.update_interval || 24;
  const intervalInput = document.getElementById('edit-sub-interval');
  if (intervalInput) intervalInput.value = interval;

  selectIntervalChip(interval);

  // Populate Quota & Expiry in Edit Modal
  const trafficLeftEl = document.getElementById('edit-sub-traffic-left');
  if (trafficLeftEl) {
    if (groupData.total) {
      const used = (groupData.upload || 0) + (groupData.download || 0);
      const left = groupData.total - used;
      trafficLeftEl.textContent = formatBytes(Math.max(0, left));
    } else {
      trafficLeftEl.textContent = '-';
    }
  }

  const daysLeftEl = document.getElementById('edit-sub-days-left');
  if (daysLeftEl) {
    if (groupData.expire) {
      const now = Math.floor(Date.now() / 1000);
      const days = Math.max(0, Math.floor((groupData.expire - now) / 86400));
      daysLeftEl.textContent = days + ' days';
    } else {
      daysLeftEl.textContent = '-';
    }
  }

  // Populate Total Used, Average Ping & Render Daily Graph
  const updateModalStats = (stats) => {
    if (!stats) return;
    const totalEl = document.getElementById('edit-sub-total-used');
    if (totalEl) totalEl.textContent = formatBytes(stats.total_used || 0);

    const pingEl = document.getElementById('edit-sub-avg-ping');
    if (pingEl) {
      const pings = stats.ping_history || [];
      if (pings.length > 0) {
        pingEl.textContent = Math.round(pings.reduce((a, b) => a + b, 0) / pings.length) + ' ms';
      } else {
        pingEl.textContent = '- ms';
      }
    }
  };

  if (groupData.stats) {
    updateModalStats(groupData.stats);
  }
  invoke('get_item_stats', { isGroup: true, key: groupKey }).then(stats => {
    if (stats) updateModalStats(stats);
  }).catch(() => {});
}

function closeEditSubModal() {
  const modal = document.getElementById('edit-sub-modal');
  if (modal) modal.style.display = 'none';
  currentEditSubKey = null;
}

async function saveSubscriptionEdits() {
  if (!currentEditSubKey) return;
  const name = document.getElementById('edit-sub-name').value.trim();
  const new_url = document.getElementById('edit-sub-url').value.trim();
  const auto_update = document.getElementById('edit-sub-auto-update').checked;
  const update_interval = parseInt(document.getElementById('edit-sub-interval').value) || 24;
  
  const urlChanged = new_url && new_url !== currentEditSubKey;

  try {
    await invoke('edit_subscription', {
      url: currentEditSubKey,
      newUrl: urlChanged ? new_url : null,
      name,
      autoUpdate: auto_update,
      updateInterval: update_interval
    });
    closeEditSubModal();
    if (urlChanged) {
      // Sync fresh configs for the updated subscription URL in background
      invoke('update_subscription', { url: new_url }).catch(e => console.error(e));
    }
    await refreshServerList();
    await refreshSubscriptions();
  } catch (e) {
    log('Failed to save subscription: ' + e);
  }
}


async function refreshStatsData() {
  if (!currentStatsKey) return;
  try {
    let aggregatedStats = { total_used: 0, daily_usage: {}, ping_history: [] };
    
    if (currentStatsIsGroup) {
      const labelEl = document.querySelector('#stats-total-used').previousElementSibling;
      if (labelEl) labelEl.textContent = 'Total used in this client';
      
      const groupStats = await invoke('get_item_stats', { isGroup: true, key: currentStatsKey });
      const hasNativeGroupStats = groupStats && (groupStats.total_used > 0 || Object.keys(groupStats.daily_usage || {}).length > 0);
      
      if (hasNativeGroupStats) {
        aggregatedStats.total_used = groupStats.total_used || 0;
        aggregatedStats.daily_usage = groupStats.daily_usage || {};
      }
      
      if (currentStatsGroupData && currentStatsGroupData.servers) {
        let allPings = [];
        
        // Fetch stats concurrently to avoid blocking the UI thread for 100+ servers
        const statPromises = currentStatsGroupData.servers.map(server => {
          const lUrl = typeof server === 'string' ? server : server.link || server.url;
          return invoke('get_item_stats', { isGroup: false, key: lUrl }).catch(() => null);
        });
        const statResults = await Promise.all(statPromises);
        
        for (const s of statResults) {
          if (s) {
            if (s.ping_history) allPings.push(...s.ping_history);
            
            // Only aggregate traffic if we don't have native group stats (legacy fallback)
            if (!hasNativeGroupStats) {
              aggregatedStats.total_used += (s.total_used || 0);
              if (s.daily_usage) {
                for (const [date, bytes] of Object.entries(s.daily_usage)) {
                  aggregatedStats.daily_usage[date] = (aggregatedStats.daily_usage[date] || 0) + bytes;
                }
              }
            }
          }
        }
        aggregatedStats.ping_history = allPings;
      }
    } else {
      const labelEl = document.querySelector('#stats-total-used').previousElementSibling;
      if (labelEl) labelEl.textContent = 'Total Used';
      
      const stats = await invoke('get_item_stats', { isGroup: false, key: currentStatsKey });
      if (stats) aggregatedStats = stats;
    }

    // Check if never connected (all stats empty)
    const hasData = (aggregatedStats.total_used && aggregatedStats.total_used > 0) || 
                    (aggregatedStats.daily_usage && Object.keys(aggregatedStats.daily_usage).length > 0) ||
                    (aggregatedStats.ping_history && aggregatedStats.ping_history.length > 0);
    
    if (!hasData) {
      document.getElementById('stats-total-used').textContent = 'Waiting for connection';
      document.getElementById('stats-avg-ping').textContent = '- ms';
    } else {
      // Update total used
      document.getElementById('stats-total-used').textContent = formatBytes(aggregatedStats.total_used || 0);
      
      // Update average ping
      const pings = aggregatedStats.ping_history || [];
      if (pings.length > 0) {
        const avg = Math.round(pings.reduce((a, b) => a + b, 0) / pings.length);
        document.getElementById('stats-avg-ping').textContent = avg + ' ms';
      } else {
        document.getElementById('stats-avg-ping').textContent = '- ms';
      }
    }
    
    // Draw graph
    drawStatsGraph(aggregatedStats.daily_usage || {}, aggregatedStats.ping_history || []);
  } catch (e) {
    console.error('Stats refresh error:', e);
  }
}

function renderDailyUsageGraph(canvas, dailyUsage) {
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  
  let dates = Object.keys(dailyUsage || {}).sort();
  
  // Ensure we have at least 2 points so it's never just a dot
  if (dates.length === 1) {
    const [y, m, d] = dates[0].split('-').map(Number);
    const yesterday = new Date(y, m - 1, d - 1);
    const yStr = `${yesterday.getFullYear()}-${String(yesterday.getMonth()+1).padStart(2,'0')}-${String(yesterday.getDate()).padStart(2,'0')}`;
    dailyUsage[yStr] = 0;
    dates.unshift(yStr);
  }
  
  const dpr = window.devicePixelRatio || 1;
  // Fix vertical stretch by reading actual CSS height
  const style = window.getComputedStyle(canvas);
  let w = parseFloat(style.width) || canvas.parentElement?.clientWidth || 670;
  let h = parseFloat(style.height) || 200;

  // Force CSS dimensions to prevent parent container from growing on high-DPI screens
  canvas.style.width = `${w}px`;
  canvas.style.height = `${h}px`;
  
  canvas.width = w * dpr;
  canvas.height = h * dpr;
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);
  
  if (dates.length === 0) {
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
    ctx.beginPath();
    ctx.moveTo(30, h - 25);
    ctx.lineTo(w - 30, h - 25);
    ctx.stroke();

    ctx.fillStyle = 'rgba(255, 255, 255, 0.4)';
    ctx.font = '12px Inter, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('No traffic data recorded yet', w / 2, h / 2);
    return;
  }
  
  const usages = dates.map(d => dailyUsage[d]);
  const maxUsage = Math.max(...usages) * 1.1 || 1;
  
  const padBottom = 25;
  const padTop = 15;
  const padLeft = 60;
  const padRight = 20;
  
  const graphW = Math.max(10, w - padLeft - padRight);
  const graphH = Math.max(10, h - padBottom - padTop);
  
  // Draw grid & Y-axis labels
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
  ctx.fillStyle = '#888';
  ctx.font = '10px Inter, sans-serif';
  ctx.textAlign = 'right';
  ctx.textBaseline = 'middle';
  
  const ySteps = 3;
  for (let i = 0; i <= ySteps; i++) {
    const yVal = maxUsage * (i / ySteps);
    const yPos = padTop + graphH - (graphH * (i / ySteps));
    ctx.beginPath();
    ctx.moveTo(padLeft, yPos);
    ctx.lineTo(padLeft + graphW, yPos);
    ctx.stroke();
    ctx.fillText(formatBytes(yVal), padLeft - 6, yPos);
  }
  
  // Draw Line Chart
  if (dates.length > 0) {
    ctx.beginPath();
    
    const stepX = graphW / (dates.length - 1);
    
    const points = dates.map((dateStr, i) => {
      const usage = usages[i];
      let px = padLeft + (i * stepX);
      const py = padTop + graphH - ((usage / maxUsage) * graphH);
      return { x: px, y: py };
    });
    
    // Gradient fill under the line
    const gradient = ctx.createLinearGradient(0, padTop, 0, padTop + graphH);
    gradient.addColorStop(0, 'rgba(74, 222, 128, 0.3)');
    gradient.addColorStop(1, 'rgba(74, 222, 128, 0.0)');
    
    ctx.beginPath();
    ctx.moveTo(points[0].x, padTop + graphH);
    for (const p of points) {
      ctx.lineTo(p.x, p.y);
    }
    ctx.lineTo(points[points.length - 1].x, padTop + graphH);
    ctx.fillStyle = gradient;
    ctx.fill();
    
    // The line itself
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (const p of points) {
      ctx.lineTo(p.x, p.y);
    }
    ctx.strokeStyle = 'rgba(74, 222, 128, 1)';
    ctx.lineWidth = 2;
    ctx.stroke();
    
    // Draw points and X-axis labels
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    ctx.fillStyle = '#888';
    
    points.forEach((p, i) => {
      // Dot
      ctx.beginPath();
      ctx.arc(p.x, p.y, 3, 0, 2 * Math.PI);
      ctx.fillStyle = '#fff';
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = 'rgba(74, 222, 128, 1)';
      ctx.stroke();
      
      // Label
      const [yr, mo, dy] = dates[i].split('-').map(Number);
      const dateObj = new Date(yr, mo - 1, dy);
      const label = dateObj.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
      
      // Only draw label if there's space (avoid crowding)
      if (stepX > 40 || i % Math.ceil(40 / stepX) === 0 || i === points.length - 1) {
        ctx.fillStyle = '#888';
        ctx.fillText(label, p.x, padTop + graphH + 8);
      }
    });
  }
}

function drawStatsGraph(dailyUsage, pingHistory) {
  renderDailyUsageGraph(document.getElementById('stats-canvas'), dailyUsage);
}

// ===== Routing Rules Display =====
async function refreshRoutingRules() {
  try {
    const result = await invoke('get_routing_rules');
    const rules = result.rules || result || [];
    const listEl = document.getElementById('routing-rules-list');
    if (!listEl) return;
    listEl.innerHTML = '';
    
    if (Array.isArray(rules)) {
      rules.forEach(rule => {
        const item = document.createElement('div');
        item.className = 'routing-rule-item';
        const isBypass = rule.action === 'bypass';
        const iconSvg = isBypass
          ? '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 14 20 9 15 4"></polyline><path d="M4 20v-7a4 4 0 0 1 4-4h12"></path></svg>'
          : '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"></line></svg>';
        const actionLabel = isBypass ? 'Bypass' : 'Block';
        const isEnabled = rule.enabled !== false; // Default to true if missing
        const patternStyle = isEnabled ? '' : 'opacity: 0.4; text-decoration: line-through;';
        
        item.innerHTML = `
          <div style="display: flex; align-items: center; gap: 8px; flex: 1; overflow: hidden;">
            <span class="routing-rule-pattern" style="${patternStyle}">${rule.pattern}</span>
            <span class="routing-rule-action ${rule.action}" style="${!isEnabled ? 'opacity: 0.4;' : ''}">${iconSvg} ${actionLabel}</span>
          </div>
          <div style="display: flex; align-items: center; gap: 8px; flex-shrink: 0;">
            <label class="toggle-switch" style="transform: scale(0.65); margin: 0;" title="Enable/Disable Rule">
              <input type="checkbox" onchange="handleToggleRoutingRule('${rule.id}', this.checked)" ${isEnabled ? 'checked' : ''}>
              <span class="toggle-slider"></span>
            </label>
            <button class="routing-rule-delete" onclick="handleDeleteRoutingRule('${rule.id}')" title="Delete Rule">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
            </button>
          </div>
        `;
        listEl.appendChild(item);
      });
    }
  } catch (e) {
    console.error('Failed to load routing rules:', e);
  }
}

async function handleToggleRoutingRule(id, enabled) {
  try {
    await invoke('toggle_routing_rule', { id: id, enabled: enabled });
    refreshRoutingRules();
    if(window.restartConnectionIfActive) await window.restartConnectionIfActive();
  } catch (e) {
    console.error('Failed to toggle routing rule:', e);
  }
}

async function handleDeleteRoutingRule(id) {
  try {
    await invoke('delete_routing_rule', { id: id });
    refreshRoutingRules();
    if(window.restartConnectionIfActive) await window.restartConnectionIfActive();
  } catch (e) {
    console.error(e);
  }
}

// ===== Subscription Collapse Memory =====
async function saveCollapsedGroupsToSettings() {
  try {
    localStorage.setItem('vxray_collapsed_groups', JSON.stringify(collapsedGroups));
  } catch (e) {}
}

function loadCollapsedGroupsFromSettings() {
  try {
    const saved = localStorage.getItem('vxray_collapsed_groups');
    if (saved) {
      const parsed = JSON.parse(saved);
      Object.assign(collapsedGroups, parsed);
    }
  } catch (e) {}
}

// ===== Auto-Update Timer =====
let autoUpdateInterval = null;

function startAutoUpdateTimer() {
  if (autoUpdateInterval) clearInterval(autoUpdateInterval);
  // Update every 4 hours (4 * 60 * 60 * 1000 = 14400000)
  autoUpdateInterval = setInterval(async () => {
    try {
      const settings = await invoke('get_startup_settings');
      if (settings && settings.auto_update_subs) {
        console.log('Auto-updating subscriptions (4h timer)...');
        await updateAllSubscriptions();
      }
    } catch (e) { console.error(e); }
  }, 14400000);
}

// ===== Enhanced Init =====
document.addEventListener("DOMContentLoaded", () => {
  refreshSubscriptions();
  loadCollapsedGroupsFromSettings();
  initRoutingPresets();
  initSettingsListeners();
  loadSettingsValues();
  refreshRoutingRules();
  startAutoUpdateTimer();
});

// ===== Batch Import & Modal Logic =====
function openImportModal() {
  const modal = document.getElementById('import-modal');
  if (modal) modal.style.display = 'flex';
  
  const textarea = document.getElementById('txt-import-content');
  const banner = document.getElementById('import-status-banner');
  if (banner) {
    banner.style.display = 'none';
    banner.textContent = '';
  }
  
  if (textarea) {
    textarea.focus();
    updateImportPreview();
  }
}

function closeImportModal() {
  const modal = document.getElementById('import-modal');
  if (modal) modal.style.display = 'none';
  const banner = document.getElementById('import-status-banner');
  if (banner) banner.style.display = 'none';
}

function clearImportText() {
  const textarea = document.getElementById('txt-import-content');
  if (textarea) {
    textarea.value = '';
    textarea.focus();
  }
  updateImportPreview();
}

async function pasteFromClipboard() {
  try {
    let text = '';
    try {
      text = await invoke('read_from_clipboard');
    } catch (_) {
      text = await navigator.clipboard.readText();
    }
    if (text) {
      const textarea = document.getElementById('txt-import-content');
      if (textarea) {
        if (textarea.value.trim().length > 0) {
          textarea.value = textarea.value.trim() + '\n\n' + text.trim();
        } else {
          textarea.value = text.trim();
        }
        textarea.focus();
        updateImportPreview();
      }
    }
  } catch (err) {
    console.warn('Clipboard read error:', err);
  }
}

function parseImportContent(rawText) {
  if (!rawText) return { servers: [], subscriptions: [] };
  let text = rawText.trim();

  // Try decoding base64 content if the block is pure base64 (e.g. copied from telegram)
  const compact = text.replace(/\s+/g, '');
  if (/^[A-Za-z0-9+/=]{40,}$/.test(compact)) {
    try {
      const decoded = atob(compact);
      if (decoded.includes('://')) {
        text = decoded;
      }
    } catch (e) {}
  }

  const lines = text.split(/[\r\n]+/);
  const serverSet = new Set();
  const subSet = new Set();
  const proxyPrefixes = ['vless://', 'vmess://', 'ss://', 'trojan://', 'hy2://', 'hysteria2://', 'tuic://'];

  for (let line of lines) {
    const trimmedLine = line.trim();
    if (!trimmedLine) continue;

    // Handle multiple tokens/links on a single line separated by whitespace
    const tokens = trimmedLine.split(/\s+/);
    for (const token of tokens) {
      const t = token.trim();
      if (!t) continue;

      const isProxy = proxyPrefixes.some(p => t.toLowerCase().startsWith(p));
      if (isProxy) {
        serverSet.add(t);
      } else if (t.startsWith('http://') || t.startsWith('https://')) {
        subSet.add(t);
      }
    }
  }

  return {
    servers: Array.from(serverSet),
    subscriptions: Array.from(subSet)
  };
}

function updateImportPreview() {
  const textarea = document.getElementById('txt-import-content');
  const badge = document.getElementById('import-preview-badge');
  const btn = document.getElementById('btn-do-import');
  if (!textarea || !badge || !btn) return;

  const { servers, subscriptions } = parseImportContent(textarea.value);
  const total = servers.length + subscriptions.length;

  if (total === 0) {
    badge.className = 'import-preview-badge';
    badge.innerHTML = '<span>Waiting for input...</span>';
    btn.disabled = true;
    btn.innerHTML = '<span>Import</span>';
  } else {
    badge.className = 'import-preview-badge has-items';
    const parts = [];
    if (servers.length > 0) parts.push(`${servers.length} server${servers.length > 1 ? 's' : ''}`);
    if (subscriptions.length > 0) parts.push(`${subscriptions.length} subscription${subscriptions.length > 1 ? 's' : ''}`);
    badge.innerHTML = `<span>âœ“ Recognized ${parts.join(', ')}</span>`;
    btn.disabled = false;
    btn.innerHTML = `<span>Import ${total} Item${total > 1 ? 's' : ''}</span>`;
  }
}

async function executeBatchImport() {
  const textarea = document.getElementById('txt-import-content');
  const btn = document.getElementById('btn-do-import');
  const banner = document.getElementById('import-status-banner');
  if (!textarea || !btn) return;

  const { servers, subscriptions } = parseImportContent(textarea.value);
  const total = servers.length + subscriptions.length;
  if (total === 0) return;

  btn.disabled = true;
  btn.innerHTML = `<span>Importing...</span>`;

  if (banner) {
    banner.style.display = 'flex';
    banner.style.background = 'rgba(255, 255, 255, 0.08)';
    banner.style.color = '#ffffff';
    banner.style.border = '1px solid rgba(255, 255, 255, 0.15)';
    banner.innerHTML = `<span>Processing ${total} items...</span>`;
  }

  let importedServers = 0;
  let importedSubs = 0;
  const errors = [];

  try {
    // 1. Import manual servers in batch
    if (servers.length > 0) {
      try {
        importedServers = await invoke('add_servers', { links: servers });
      } catch (e) {
        console.error('Failed to add servers in batch, falling back to sequential:', e);
        for (const s of servers) {
          try {
            await invoke('add_server', { link: s });
            importedServers++;
          } catch (err) {
            errors.push(err);
          }
        }
      }
    }

    // 2. Import subscriptions and immediately update them
    if (subscriptions.length > 0) {
      for (const subUrl of subscriptions) {
        try {
          await invoke('add_subscription', { url: subUrl });
          await invoke('update_subscription', { url: subUrl });
          importedSubs++;
        } catch (e) {
          console.error(`Failed to add/update subscription ${subUrl}:`, e);
          errors.push(e);
        }
      }
    }

    // 3. Refresh UI
    await refreshServerList();
    await refreshSubscriptions();

    if (banner) {
      banner.style.background = 'rgba(74, 222, 128, 0.15)';
      banner.style.color = '#4ade80';
      banner.style.border = '1px solid rgba(74, 222, 128, 0.3)';
      const msgParts = [];
      if (importedServers > 0) msgParts.push(`${importedServers} server(s)`);
      if (importedSubs > 0) msgParts.push(`${importedSubs} subscription(s)`);
      banner.innerHTML = `<span>âœ“ Successfully imported ${msgParts.join(' and ')}!</span>`;
    }

    // Auto-close after brief delay
    setTimeout(() => {
      closeImportModal();
      textarea.value = '';
      updateImportPreview();
      // If only subscriptions were added, switch to Subscriptions tab; otherwise Server tab
      if (importedServers > 0) {
        switchTab(0);
      } else if (importedSubs > 0) {
        switchTab(1);
      }
    }, 1200);

  } catch (err) {
    console.error('Batch import failed:', err);
    if (banner) {
      banner.style.background = 'rgba(248, 113, 113, 0.15)';
      banner.style.color = '#f87171';
      banner.style.border = '1px solid rgba(248, 113, 113, 0.3)';
      banner.innerHTML = `<span>Import failed: ${err}</span>`;
    }
    btn.disabled = false;
    btn.innerHTML = `<span>Try Again</span>`;
  }
}

// ===== Window Exposures =====
window.switchSettingsTab = switchSettingsTab;
window.closeStatsModal = closeStatsModal;
window.closeEditSubModal = closeEditSubModal;
window.saveSubscriptionEdits = saveSubscriptionEdits;
window.handleDeleteRoutingRule = handleDeleteRoutingRule;
window.openEditSubModal = openEditSubModal;
window.selectIntervalChip = selectIntervalChip;
window.copyCurrentEditUrl = copyCurrentEditUrl;
window.toggleCardStats = toggleCardStats;
window.updateSingleSubscriptionCard = updateSingleSubscriptionCard;
window.deleteSubscriptionFromTab = deleteSubscriptionFromTab;
window.updateAllSubscriptions = updateAllSubscriptions;
window.refreshSubscriptions = refreshSubscriptions;
window.openImportModal = openImportModal;
window.closeImportModal = closeImportModal;
window.clearImportText = clearImportText;
window.pasteFromClipboard = pasteFromClipboard;
window.updateImportPreview = updateImportPreview;
window.executeBatchImport = executeBatchImport;
window.handleGlobalPasteShortcut = handleGlobalPasteShortcut;
window.triggerPingTestShortcut = triggerPingTestShortcut;
window.triggerCopyShortcut = triggerCopyShortcut;
window.showServerCopiedFeedback = showServerCopiedFeedback;
window.closeEditServerModal = closeEditServerModal;
window.saveServerEdits = saveServerEdits;


// ===== Help System =====
let _helpCurrentTopic = '';
let _helpCurrentLang = 'en';

const helpContent = {
  "tls_fragment": {
    "en": {
      "title": "TLS Fragment",
      "body": "<p>When enabled, the TLS ClientHello packets are fragmented into smaller pieces before reaching the firewall. This is highly effective at bypassing deep packet inspection (DPI) and SNI blocking.</p>"
    },
    "fa": {
      "title": "قطعه‌قطعه کردن TLS (Fragment)",
      "body": "<p>با فعال‌سازی این گزینه، بسته‌های اولیه اتصال (ClientHello) قبل از رسیدن به فایروال به قطعات کوچکتر تقسیم می‌شوند. این ویژگی برای دور زدن سیستم‌های فیلترینگ پیشرفته (DPI) و جلوگیری از مسدود شدن دامنه (SNI) بسیار موثر است.</p>"
    }
  },
  "auto_start": {
    "en": {
      "title": "Auto Start",
      "body": "<p>When enabled, the application will automatically launch in the background when you turn on your computer.</p>"
    },
    "fa": {
      "title": "اجرای خودکار",
      "body": "<p>با فعال‌سازی این گزینه، نرم‌افزار به محض روشن شدن سیستم شما به صورت خودکار در پس‌زمینه اجرا خواهد شد.</p>"
    }
  },
  "start_minimized": {
    "en": {
      "title": "Start Minimized",
      "body": "<p>If Auto Start is on, enabling this will cause the app to launch silently into the system tray without popping up a window.</p>"
    },
    "fa": {
      "title": "شروع در حالت پنهان",
      "body": "<p>در صورت فعال بودن اجرای خودکار، این گزینه باعث می‌شود برنامه پس از روشن شدن سیستم، بدون باز کردن پنجره و به صورت خاموش در System Tray ویندوز قرار بگیرد.</p>"
    }
  },
  "auto_update_subs": {
    "en": {
      "title": "Auto Update Subscriptions",
      "body": "<p>When enabled, your subscriptions will be automatically refreshed in the background every few hours to ensure you always have active nodes.</p>"
    },
    "fa": {
      "title": "بروزرسانی خودکار اشتراک‌ها",
      "body": "<p>با فعال‌سازی این گزینه، لینک‌های اشتراک شما به صورت دوره‌ای و خودکار در پس‌زمینه بروزرسانی می‌شوند تا همیشه به سرورهای جدید و فعال دسترسی داشته باشید.</p>"
    }
  },
  "auto_reconnect": {
    "en": {
      "title": "Auto Reconnect",
      "body": "<p>If the connection drops or the app is restarted, it will automatically attempt to reconnect to your last active server and restore your previous connection mode (System Proxy or TUN).</p>"
    },
    "fa": {
      "title": "اتصال مجدد خودکار",
      "body": "<p>در صورتی که ارتباط قطع شود یا برنامه را ببندید و دوباره باز کنید، نرم‌افزار به طور خودکار به آخرین سروری که به آن متصل بودید وصل شده و حالت قبلی (پروکسی سیستم یا TUN) را بازیابی می‌کند.</p>"
    }
  },
  "live_ping": {
    "en": {
      "title": "Live Ping",
      "body": "<p>Enables real-time connection monitoring in the dashboard, showing your active ping and network health while connected.</p>"
    },
    "fa": {
      "title": "پینگ زنده",
      "body": "<p>این گزینه وضعیت اتصال و پینگ لحظه‌ای شما را به صورت زنده در داشبورد نرم‌افزار نمایش می‌دهد تا از کیفیت ارتباط خود مطلع باشید.</p>"
    }
  },
  "routing": {
    "en": {
      "title": "Routing Rules",
      "body": "<p><strong>Routing Rules</strong> determine how your network traffic is handled by the VPN.</p><br><p><strong>How to use Custom Rules:</strong></p><ol><li><strong>Domains & IPs:</strong> You can add specific domain names (e.g., <code>example.com</code>) or IP addresses (e.g., <code>192.168.1.0/24</code>) separated by commas.</li><li><strong>Proxy:</strong> Forces the specified websites to go through your VPN server.</li><li><strong>Bypass (Direct):</strong> Sends the traffic directly through your normal internet provider, completely bypassing the VPN. Highly recommended for local banking apps or online games.</li><li><strong>Block:</strong> Completely blocks access to the entered domains or IPs, which is useful for blocking trackers or malware.</li></ol>"
    },
    "fa": {
      "title": "قوانین مسیریابی (Routing)",
      "body": "<p><strong>قوانین مسیریابی</strong> تعیین می‌کنند که ترافیک اینترنت شما چگونه توسط برنامه مدیریت شود.</p><br><p><strong>آموزش استفاده از قوانین شخصی (Custom Rules):</strong></p><ol><li><strong>دامنه‌ها و آی‌پی‌ها:</strong> شما می‌توانید آدرس سایت‌ها (مانند <code>example.com</code>) یا آدرس‌های IP (مانند <code>192.168.1.0/24</code>) را با کاما (,) از هم جدا کرده و وارد کنید.</li><li><strong>پروکسی (Proxy):</strong> ترافیک سایت‌های وارد شده را مجبور می‌کند تا حتماً از سرور فیلترشکن شما عبور کنند.</li><li><strong>عبور (Bypass):</strong> ترافیک را به طور کامل از فیلترشکن خارج کرده و از اینترنت عادی شما عبور می‌دهد. این حالت برای باز کردن سایت‌های بانک‌های ایرانی یا کاهش پینگ در بازی‌های آنلاین بسیار کاربردی است.</li><li><strong>مسدود کردن (Block):</strong> دسترسی به سایت‌ها یا آی‌پی‌های وارد شده را کاملاً قطع می‌کند. از این گزینه می‌توانید برای مسدودسازی تبلیغات یا بدافزارها استفاده کنید.</li></ol>"
    }
  },
  "subscriptions": {
    "en": {
      "title": "Subscriptions",
      "body": "<p><strong>Subscriptions</strong> are links provided by your VPN provider that contain a list of servers.</p><p><strong>Auto-Update:</strong> You can edit a subscription to set an auto-update timer (e.g., every 12 hours) to always keep it fresh.</p><p><strong>Statistics:</strong> Click on the statistics row of a subscription (delay/traffic) to view detailed charts showing your usage, average ping, and remaining subscription days.</p>"
    },
    "fa": {
      "title": "اشتراک‌ها (Subscriptions)",
      "body": "<p><strong>اشتراک‌ها</strong> لینک‌هایی هستند که توسط ارائه‌دهنده سرویس به شما داده می‌شوند و شامل لیستی از سرورها هستند.</p><p><strong>بروزرسانی خودکار:</strong> با ویرایش یک اشتراک می‌توانید برای آن تایمر تعیین کنید (مثلاً هر 12 ساعت) تا سرورها به صورت خودکار بروز شوند.</p><p><strong>آمار مصرف:</strong> با کلیک روی بخش آمار اشتراک (آیکون‌های ترافیک و پینگ)، می‌توانید نمودار مصرف حجم، میانگین پینگ و روزهای باقیمانده از اشتراک خود را با جزئیات کامل مشاهده کنید.</p>"
    }
  },
  "bypass_iran": {
    "en": {
      "title": "Bypass Iran Sites",
      "body": "<p>When enabled, all traffic to Iranian websites and IP ranges goes through your direct connection instead of the VPN.</p><p>This is extremely helpful for local banking apps and domestic websites that require a direct connection, greatly improving loading speeds.</p>"
    },
    "fa": {
      "title": "دور زدن سایت‌های ایرانی",
      "body": "<p>با فعال‌سازی این گزینه، تمامی ترافیک به سمت سایت‌ها و دامنه‌های ایرانی مستقیماً از اینترنت اصلی شما (بدون عبور از فیلترشکن) ارسال می‌شود.</p><p>این ویژگی برای استفاده همزمان از اپلیکیشن‌های بانکی و سایت‌های داخلی بسیار مفید است و سرعت باز شدن آن‌ها را به شدت افزایش می‌دهد.</p>"
    }
  },
  "bypass_lan": {
    "en": {
      "title": "Bypass LAN",
      "body": "<p>When enabled, traffic directed to your Local Area Network addresses (like 192.168.x.x) will bypass the VPN tunnel.</p><p>This allows you to still access your home smart devices, printers, or local servers while keeping the VPN active.</p>"
    },
    "fa": {
      "title": "دور زدن شبکه محلی (LAN)",
      "body": "<p>با فعال کردن این گزینه، ارتباط شما با دستگاه‌های شبکه داخلی (آی‌پی‌های مانند 192.168.x.x) از فیلترشکن عبور نمی‌کند.</p><p>این به شما اجازه می‌دهد تا در حین اتصال به فیلترشکن، همچنان بتوانید به مودم، پرینترها یا دستگاه‌های هوشمند خانگی خود دسترسی داشته باشید.</p>"
    }
  },
  "block_ads": {
    "en": {
      "title": "Block Ads",
      "body": "<p>When enabled, commonly known advertising and tracking domains are blocked completely. This provides basic network-level ad filtering, speeds up page loads, and protects your privacy.</p>"
    },
    "fa": {
      "title": "مسدودسازی تبلیغات",
      "body": "<p>با انتخاب این گزینه، دامنه‌های معروف تبلیغاتی و ردیاب‌ها به طور کامل در سطح شبکه مسدود می‌شوند. این ویژگی باعث افزایش سرعت بارگذاری صفحات وب شده و از حریم خصوصی شما محافظت می‌کند.</p>"
    }
  },
  "strict_route": {
    "en": {
      "title": "Strict Route",
      "body": "<p>When enabled in TUN mode, strict route ensures that ALL system traffic is forcibly captured by the VPN tunnel, preventing any application from intentionally bypassing the tunnel and leaking your IP.</p>"
    },
    "fa": {
      "title": "مسیریابی سخت‌گیرانه (Strict Route)",
      "body": "<p>وقتی در حالت TUN متصل هستید، این گزینه سیستم را مجبور می‌کند تا ۱۰۰٪ ترافیک تمامی برنامه‌ها را به داخل تونل فیلترشکن هدایت کند و جلوی نشت آی‌پی (IP Leak) یا فرار برنامه‌ها از فیلترشکن را به طور کامل می‌گیرد.</p>"
    }
  },
  "remote_dns": {
    "en": {
      "title": "Remote Secure DNS",
      "body": "<p>This is the DNS server used to resolve domain names inside the VPN tunnel (e.g., <code>https://1.1.1.1/dns-query</code>). It encrypts your queries via HTTPS to prevent your ISP from eavesdropping.</p>"
    },
    "fa": {
      "title": "دی‌ان‌اس امن خارجی",
      "body": "<p>این سرور وظیفه ترجمه آدرس سایت‌ها در داخل تونل فیلترشکن را بر عهده دارد (مثلاً <code>https://1.1.1.1/dns-query</code>). با رمزنگاری درخواست‌های شما، مانع از شنود و ردیابی سایت‌های بازدید شده توسط شرکت ارائه‌دهنده اینترنت شما می‌شود.</p>"
    }
  },
  "local_dns": {
    "en": {
      "title": "Direct Local DNS",
      "body": "<p>This DNS is used for traffic that bypasses the VPN directly. The default <code>local</code> uses your system's built-in DNS. You can change this to <code>8.8.8.8</code> or others if you want a specific resolver for bypassed traffic.</p>"
    },
    "fa": {
      "title": "دی‌ان‌اس محلی",
      "body": "<p>این سرور برای ترافیک‌هایی استفاده می‌شود که از فیلترشکن عبور نمی‌کنند (ترافیک‌های Bypass). مقدار پیش‌فرض <code>local</code> از دی‌ان‌اس خود ویندوز استفاده می‌کند. در صورت نیاز به یک دی‌ان‌اس خاص برای سایت‌های داخلی، می‌توانید آن را به مقادیری مثل <code>8.8.8.8</code> تغییر دهید.</p>"
    }
  },
  "core_selection": {
    "en": {
      "title": "Core Selection",
      "body": "<p>Vxray supports two proxy engines (cores):</p><p><strong>Auto</strong> – The app will automatically select the best core for the node.</p><p><strong>Sing-Box</strong> – A modern, highly-performant core that is the default choice.</p><p><strong>Xray</strong> – A mature core with extensive protocol support, kept for advanced setups like XHTTP.</p>"
    },
    "fa": {
      "title": "انتخاب هسته (Core)",
      "body": "<p>برنامه Vxray از دو موتور پردازشی قدرتمند پشتیبانی می‌کند:</p><p><strong>خودکار (Auto)</strong> – نرم‌افزار به صورت هوشمند بهترین هسته را بر اساس نوع سرور شما انتخاب می‌کند.</p><p><strong>Sing-Box</strong> – یک هسته مدرن و فوق‌العاده سریع که انتخاب اصلی سیستم است.</p><p><strong>Xray</strong> – یک هسته باسابقه با پشتیبانی گسترده از پروتکل‌ها، ایده‌آل برای پیکربندی‌های پیشرفته مانند XHTTP.</p>"
    }
  }
};;

function showHelp(topic) {
  _helpCurrentTopic = topic;
  const content = helpContent[topic];
  if (!content) return;
  
  const modal = document.getElementById('help-modal');
  modal.style.display = 'flex';
  
  renderHelpContent(content, _helpCurrentLang);
  
  // Update lang buttons
  document.querySelectorAll('.help-lang-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.lang === _helpCurrentLang);
  });
}

function switchHelpLang(lang) {
  _helpCurrentLang = lang;
  const content = helpContent[_helpCurrentTopic];
  if (!content) return;
  
  renderHelpContent(content, lang);
  
  document.querySelectorAll('.help-lang-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.lang === lang);
  });
}

function renderHelpContent(content, lang) {
  const body = document.getElementById('help-modal-body');
  const data = content[lang] || content['en'];
  const isRTL = lang === 'fa';
  
  body.innerHTML = `
    <div class="${isRTL ? 'help-rtl' : ''}">
      <div class="help-title">${data.title}</div>
      ${data.body}
    </div>
  `;
}

function closeHelp() {
  document.getElementById('help-modal').style.display = 'none';
}

// Close help modal on backdrop click
document.addEventListener('click', (e) => {
  const modal = document.getElementById('help-modal');
  if (e.target === modal) {
    closeHelp();
  }
});

// Close help modal on Escape
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    const modal = document.getElementById('help-modal');
    if (modal && modal.style.display !== 'none') {
      closeHelp();
    }
  }
});

window.showHelp = showHelp;
window.switchHelpLang = switchHelpLang;
window.closeHelp = closeHelp;

// -----------------------------------------
// SPEED TEST MODULE
// -----------------------------------------

let stActive = false;
let stAbortController = null;

async function startSpeedTest() {
  const btnGo = document.getElementById('st-btn-go-large');
  const btnStop = document.getElementById('st-btn-stop');
  const gaugeArea = document.getElementById('st-gauge-area');
  const resultsArea = document.getElementById('st-results');
  
  if (stActive) {
    if (stAbortController) {
      stAbortController.abort();
      stAbortController = null;
    }
    return;
  }
  
  stActive = true;
  stAbortController = new AbortController();
  const signal = stAbortController.signal;
  
  // Transition UI: Hide GO, Show Gauge & STOP
  btnGo.style.opacity = '0';
  btnGo.style.pointerEvents = 'none';
  
  setTimeout(() => {
      btnGo.style.display = 'none';
      gaugeArea.style.display = 'flex';
      btnStop.style.display = 'block';
      setTimeout(() => {
          gaugeArea.style.opacity = '1';
          resultsArea.style.opacity = '1';
      }, 50);
  }, 300);

  document.getElementById('st-val-ping').textContent = '---';
  document.getElementById('st-val-download').textContent = '---';
  document.getElementById('st-val-upload').textContent = '---';
  setGauge(0);
  
  // Cloudflare endpoints
  let dlUrl = 'https://speed.cloudflare.com/__down?bytes=500000000'; // 500MB to guarantee 5s on Gigabit
  let ulUrl = 'https://speed.cloudflare.com/__up';
  let pingUrl = 'https://cloudflare.com/cdn-cgi/trace'; // Ultra-lightweight endpoint for accurate HTTP ping
  
  try {
    const startPing = performance.now();
    let pingMs;
    
    if (isRunning && connectedServerLink) {
        try {
            const rawLat = await invoke('get_live_ping', { link: connectedServerLink });
            pingMs = typeof rawLat === 'number' ? rawLat : parseInt(rawLat);
        } catch (_) {
            pingMs = 'Error';
        }
    } else {
        // Direct local network ping (simulating the app's ping system but without the proxy)
        try {
            await fetch(`https://www.google.com/generate_204?nocache=${Date.now()}`, { cache: 'no-store', signal });
            pingMs = Math.round(performance.now() - startPing);
        } catch (_) {
            pingMs = 'Error';
        }
    }
    
    document.getElementById('st-val-ping').textContent = (typeof pingMs === 'number' ? pingMs + ' ms' : pingMs);
    
    // Download Test (5 seconds)
    const dlStart = performance.now();
    let receivedLength = 0;
    try {
      const response = await fetch(dlUrl, { cache: 'no-store', signal });
      const reader = response.body.getReader();
      
      while(true) {
        const {done, value} = await reader.read();
        if (done) break;
        receivedLength += value.length;
        
        const currentMs = performance.now() - dlStart;
        if (currentMs > 100) {
            const speedMbps = ((receivedLength / (currentMs / 1000)) * 8) / 1000000;
            setGauge(speedMbps);
            document.getElementById('st-val-download').textContent = speedMbps.toFixed(2);
        }
        
        // Stop exactly after 8 seconds
        if (currentMs >= 8000) {
            reader.cancel();
            break;
        }
      }
    } catch (e) {
      if (e.name !== 'AbortError') throw e;
    }
    
    if (signal.aborted) throw new Error('Aborted');
    
    await new Promise(r => setTimeout(r, 1000));
    if (signal.aborted) throw new Error('Aborted');
    setGauge(0);
    
    // Upload Test (7 seconds)
    const ulStart = performance.now();
    const dummyData = new Uint8Array(200000000); // 200MB payload to guarantee 7s on Gigabit
    
    // For upload, we simulate progress by sending multiple smaller chunks, or since we can't track XHR upload progress easily with fetch, we'll just send a large payload and track time. 
    // Actually, `fetch` doesn't support upload progress. Let's use XMLHttpRequest to track upload progress properly.
    await new Promise((resolve, reject) => {
        const xhr = new XMLHttpRequest();
        xhr.open('POST', ulUrl, true);
        
        // Handle abort
        signal.addEventListener('abort', () => {
            xhr.abort();
            reject(new Error('Aborted'));
        });
        
        xhr.upload.onprogress = (e) => {
            const currentMs = performance.now() - ulStart;
            if (currentMs > 100) {
                const speedMbps = ((e.loaded / (currentMs / 1000)) * 8) / 1000000;
                setGauge(speedMbps);
                document.getElementById('st-val-upload').textContent = speedMbps.toFixed(2);
            }
            // Abort exactly after 8 seconds
            if (currentMs >= 8000) {
                xhr.abort();
                resolve();
            }
        };
        
        xhr.onload = () => resolve();
        xhr.onerror = () => reject(new Error('Network Error'));
        
        xhr.send(dummyData);
    });
  } catch (e) {
    if (e.name === 'AbortError' || e.message === 'Aborted') {
      log('Speed test stopped by user.');
    } else {
      log('Speed test failed: ' + e);
    }
    setGauge(0);
  } finally {
    stActive = false;
    stAbortController = null;
    
    // Reset gauge to zero visually
    setGauge(0);
    document.getElementById('st-speed-value').textContent = '0.00';
    
    // Transition UI back to GO button
    setTimeout(() => {
        gaugeArea.style.opacity = '0';
        btnStop.style.display = 'none';
        
        setTimeout(() => {
            gaugeArea.style.display = 'none';
            btnGo.style.display = 'block';
            setTimeout(() => {
                btnGo.style.opacity = '1';
                btnGo.style.pointerEvents = 'auto';
            }, 50);
        }, 500);
    }, 1500); // Wait 1.5 seconds so user can see it hit 0 before vanishing
  }
}

function setGauge(mbps) {
  const maxMbps = 100;
  const val = Math.min(mbps, maxMbps);
  const pct = val / maxMbps;
  const offset = 251 - (251 * pct);
  const elFill = document.getElementById('st-gauge-fill');
  if (elFill) elFill.style.strokeDashoffset = offset;
  
  const deg = -90 + (180 * pct);
  const elNeedle = document.getElementById('st-gauge-needle');
  if (elNeedle) elNeedle.style.transform = 'rotate(' + deg + 'deg)';
  
  const elVal = document.getElementById('st-speed-value');
  if (elVal) elVal.textContent = mbps.toFixed(2);
}

















