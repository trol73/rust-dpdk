use std::ptr;
use std::mem;
use std::ops::Range;
use std::ffi::CStr;
use std::os::raw::c_void;

use libc;

use ffi;

use errors::{Error, Result};
use memory::{SocketId, AsMutRef};
use mempool;
use malloc;
use mbuf;
use ether;
use pci;

pub type PortId = u8;
pub type QueueId = u16;

/// A structure used to retrieve link-level information of an Ethernet port.
pub struct EthLink {
    pub speed: u32,
    pub duplex: bool,
    pub autoneg: bool,
    pub up: bool,
}

pub trait EthDevice {
    fn portid(&self) -> PortId;

    /// Configure an Ethernet device.
    ///
    /// This function must be invoked first before any other function in the Ethernet API.
    /// This function can also be re-invoked when a device is in the stopped state.
    ///
    fn configure(&self,
                 nb_rx_queue: QueueId,
                 nb_tx_queue: QueueId,
                 conf: &EthConf)
                 -> Result<&Self>;

    /// Retrieve the contextual information of an Ethernet device.
    fn info(&self) -> RawEthDeviceInfo;

    /// Retrieve the general I/O statistics of an Ethernet device.
    fn stats(&self) -> Result<RawEthDeviceStats>;

    /// Reset the general I/O statistics of an Ethernet device.
    fn reset_stats(&self) -> &Self;

    /// Retrieve the Ethernet address of an Ethernet device.
    fn mac_addr(&self) -> ether::EtherAddr;

    /// Set the default MAC address.
    fn set_mac_addr(&self, addr: &[u8; ether::ETHER_ADDR_LEN]) -> Result<&Self>;

    /// Return the NUMA socket to which an Ethernet device is connected
    fn socket_id(&self) -> SocketId;

    /// Check if port_id of device is attached
    fn is_valid(&self) -> bool;

    /// Allocate and set up a receive queue for an Ethernet device.
    ///
    /// The function allocates a contiguous block of memory for *nb_rx_desc*
    /// receive descriptors from a memory zone associated with *socket_id*
    /// and initializes each receive descriptor with a network buffer allocated
    /// from the memory pool *mb_pool*.
    fn rx_queue_setup(&self,
                      rx_queue_id: QueueId,
                      nb_rx_desc: u16,
                      rx_conf: Option<ffi::Struct_rte_eth_rxconf>,
                      mb_pool: &mut mempool::RawMemoryPool)
                      -> Result<&Self>;

    /// Allocate and set up a transmit queue for an Ethernet device.
    fn tx_queue_setup(&self,
                      tx_queue_id: QueueId,
                      nb_tx_desc: u16,
                      tx_conf: Option<ffi::Struct_rte_eth_txconf>)
                      -> Result<&Self>;

    /// Enable receipt in promiscuous mode for an Ethernet device.
    fn promiscuous_enable(&self) -> &Self;

    /// Disable receipt in promiscuous mode for an Ethernet device.
    fn promiscuous_disable(&self) -> &Self;

    /// Return the value of promiscuous mode for an Ethernet device.
    fn is_promiscuous_enabled(&self) -> Result<bool>;

    /// Retrieve the MTU of an Ethernet device.
    fn mtu(&self) -> Result<u16>;

    /// Change the MTU of an Ethernet device.
    fn set_mtu(&self, mtu: u16) -> Result<&Self>;

    /// Enable/Disable hardware filtering by an Ethernet device
    /// of received VLAN packets tagged with a given VLAN Tag Identifier.
    fn set_vlan_filter(&self, vlan_id: u16, on: bool) -> Result<&Self>;

    /// Retrieve the Ethernet device link status
    #[inline]
    fn is_up(&self) -> bool {
        self.link().up
    }

    /// Retrieve the status (ON/OFF), the speed (in Mbps) and
    /// the mode (HALF-DUPLEX or FULL-DUPLEX) of the physical link of an Ethernet device.
    ///
    /// It might need to wait up to 9 seconds in it.
    ///
    fn link(&self) -> EthLink;

    /// Retrieve the status (ON/OFF), the speed (in Mbps) and
    /// the mode (HALF-DUPLEX or FULL-DUPLEX) of the physical link of an Ethernet device.
    ///
    /// It is a no-wait version of rte_eth_link_get().
    ///
    fn link_nowait(&self) -> EthLink;

    /// Link up an Ethernet device.
    fn set_link_up(&self) -> Result<&Self>;

    /// Link down an Ethernet device.
    fn set_link_down(&self) -> Result<&Self>;

    /// Allocate mbuf from mempool, setup the DMA physical address
    /// and then start RX for specified queue of a port. It is used
    /// when rx_deferred_start flag of the specified queue is true.
    fn rx_queue_start(&self, rx_queue_id: QueueId) -> Result<&Self>;

    /// Stop specified RX queue of a port
    fn rx_queue_stop(&self, rx_queue_id: QueueId) -> Result<&Self>;

    /// Start TX for specified queue of a port.
    /// It is used when tx_deferred_start flag of the specified queue is true.
    fn tx_queue_start(&self, tx_queue_id: QueueId) -> Result<&Self>;

    /// Stop specified TX queue of a port
    fn tx_queue_stop(&self, tx_queue_id: QueueId) -> Result<&Self>;

    /// Start an Ethernet device.
    fn start(&self) -> Result<&Self>;

    /// Stop an Ethernet device.
    fn stop(&self) -> &Self;

    /// Close a stopped Ethernet device. The device cannot be restarted!
    fn close(&self) -> &Self;

    /// Retrieve a burst of input packets from a receive queue of an Ethernet device.
    fn rx_burst(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr]) -> usize;

    /// Send a burst of output packets on a transmit queue of an Ethernet device.
    fn tx_burst(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr]) -> usize;

    fn rx_burst_ex(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr], packets: u16) -> u16;
    fn tx_burst_ex(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr], packets: u16) -> u16;

    /// Set RX L2 Filtering mode of a VF of an Ethernet device.
    fn set_vf_rxmode(&self, vf: u16, rx_mode: EthVmdqRxMode, on: bool) -> Result<&Self>;

    /// Enable or disable a VF traffic transmit of the Ethernet device.
    fn set_vf_tx(&self, vf: u16, on: bool) -> Result<&Self>;

    /// Enable or disable a VF traffic receive of an Ethernet device.
    fn set_vf_rx(&self, vf: u16, on: bool) -> Result<&Self>;

    /// Read VLAN Offload configuration from an Ethernet device
    fn vlan_offload(&self) -> Result<EthVlanOffloadMode>;

    /// Set VLAN offload configuration on an Ethernet device
    fn set_vlan_offload(&self, mode: EthVlanOffloadMode) -> Result<&Self>;
}

/// Get the total number of Ethernet devices that have been successfully initialized
/// by the matching Ethernet driver during the PCI probing phase.
///
/// All devices whose port identifier is in the range [0, rte::ethdev::count() - 1]
/// can be operated on by network applications immediately after invoking rte_eal_init().
/// If the application unplugs a port using hotplug function,
/// The enabled port numbers may be noncontiguous.
/// In the case, the applications need to manage enabled port by themselves.
pub fn count() -> u8 {
    unsafe { ffi::rte_eth_dev_count() }
}

pub fn devices() -> Range<PortId> {
    0..count()
}

/// Attach a new Ethernet device specified by aruguments.
pub fn attach(devargs: &str) -> Result<PortId> {
    let mut portid: u8 = 0;

    let ret = unsafe { ffi::rte_eth_dev_attach(try!(to_cptr!(devargs)), &mut portid) };

    rte_check!(ret; ok => { portid })
}

impl EthDevice for PortId {
    fn portid(&self) -> PortId {
        *self
    }

    fn configure(&self,
                 nb_rx_queue: QueueId,
                 nb_tx_queue: QueueId,
                 conf: &EthConf)
                 -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_configure(*self,
                                       nb_rx_queue,
                                       nb_tx_queue,
                                       RawEthConf::from(conf).as_raw())
        }; ok => { self })
    }

    fn info(&self) -> RawEthDeviceInfo {
        let mut info: RawEthDeviceInfo = Default::default();

        unsafe { ffi::rte_eth_dev_info_get(*self, &mut info) }

        info
    }

    fn stats(&self) -> Result<RawEthDeviceStats> {
        let mut stats: RawEthDeviceStats = Default::default();

        rte_check!(unsafe {
            ffi::rte_eth_stats_get(*self, &mut stats)
        }; ok => { stats })
    }

    fn reset_stats(&self) -> &Self {
        unsafe { ffi::rte_eth_stats_reset(*self) };

        self
    }

    fn mac_addr(&self) -> ether::EtherAddr {
        unsafe {
            let mut addr: ffi::Struct_ether_addr = mem::zeroed();

            ffi::rte_eth_macaddr_get(*self, &mut addr);

            ether::EtherAddr::from(addr.addr_bytes)
        }
    }

    fn set_mac_addr(&self, addr: &[u8; ether::ETHER_ADDR_LEN]) -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_default_mac_addr_set(*self, mem::transmute(addr.as_ptr()))
        }; ok => { self })
    }

    fn socket_id(&self) -> SocketId {
        unsafe { ffi::rte_eth_dev_socket_id(*self) }
    }

    fn is_valid(&self) -> bool {
        unsafe { ffi::rte_eth_dev_is_valid_port(*self) != 0 }
    }

    fn rx_queue_setup(&self,
                      rx_queue_id: QueueId,
                      nb_rx_desc: u16,
                      rx_conf: Option<ffi::Struct_rte_eth_rxconf>,
                      mb_pool: &mut mempool::RawMemoryPool)
                      -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_rx_queue_setup(*self,
                                        rx_queue_id,
                                        nb_rx_desc,
                                        self.socket_id() as u32,
                                        mem::transmute(&rx_conf),
                                        mb_pool)
        }; ok => { self })
    }

    fn tx_queue_setup(&self,
                      tx_queue_id: QueueId,
                      nb_tx_desc: u16,
                      tx_conf: Option<ffi::Struct_rte_eth_txconf>)
                      -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_tx_queue_setup(*self,
                                        tx_queue_id,
                                        nb_tx_desc,
                                        self.socket_id() as u32,
                                        mem::transmute(&tx_conf))
        }; ok => { self })
    }

    fn promiscuous_enable(&self) -> &Self {
        unsafe { ffi::rte_eth_promiscuous_enable(*self) };

        self
    }

    fn promiscuous_disable(&self) -> &Self {
        unsafe { ffi::rte_eth_promiscuous_disable(*self) };

        self
    }

    fn is_promiscuous_enabled(&self) -> Result<bool> {
        let ret = unsafe { ffi::rte_eth_promiscuous_get(*self) };

        rte_check!(ret; ok => { ret != 0 })
    }

    fn mtu(&self) -> Result<u16> {
        let mut mtu: u16 = 0;

        rte_check!(unsafe { ffi::rte_eth_dev_get_mtu(*self, &mut mtu)}; ok => { mtu })
    }

    fn set_mtu(&self, mtu: u16) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_set_mtu(*self, mtu) }; ok => { self })
    }

    fn set_vlan_filter(&self, vlan_id: u16, on: bool) -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_vlan_filter(*self, vlan_id, bool_value!(on) as i32)
        }; ok => { self })
    }

    fn link(&self) -> EthLink {
        let link = 0u64;

        unsafe { ffi::rte_eth_link_get(*self, mem::transmute(&link)) }

        EthLink {
            speed: (link & 0xFFFFFFFF) as u32,
            duplex: (link & (1 << 32)) != 0,
            autoneg: (link & (1 << 33)) != 0,
            up: (link & (1 << 34)) != 0,
        }
    }

    fn link_nowait(&self) -> EthLink {
        let link = 0u64;

        unsafe { ffi::rte_eth_link_get_nowait(*self, mem::transmute(&link)) }

        EthLink {
            speed: (link & 0xFFFFFFFF) as u32,
            duplex: (link & (1 << 32)) != 0,
            autoneg: (link & (1 << 33)) != 0,
            up: (link & (1 << 34)) != 0,
        }
    }

    fn set_link_up(&self) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_set_link_up(*self) }; ok => { self })
    }

    fn set_link_down(&self) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_set_link_down(*self) }; ok => { self })
    }

    fn rx_queue_start(&self, rx_queue_id: QueueId) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_rx_queue_start(*self, rx_queue_id) }; ok => { self })
    }

    fn rx_queue_stop(&self, rx_queue_id: QueueId) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_rx_queue_stop(*self, rx_queue_id) }; ok => { self })
    }

    fn tx_queue_start(&self, tx_queue_id: QueueId) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_tx_queue_start(*self, tx_queue_id) }; ok => { self })
    }

    fn tx_queue_stop(&self, tx_queue_id: QueueId) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_tx_queue_stop(*self, tx_queue_id) }; ok => { self })
    }

    fn start(&self) -> Result<&Self> {
        rte_check!(unsafe { ffi::rte_eth_dev_start(*self) }; ok => { self })
    }

    fn stop(&self) -> &Self {
        unsafe { ffi::rte_eth_dev_stop(*self) };

        self
    }

    fn close(&self) -> &Self {
        unsafe { ffi::rte_eth_dev_close(*self) };

        self
    }

    fn rx_burst(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr]) -> usize {
        unsafe {
            _rte_eth_rx_burst(*self, queue_id, rx_pkts.as_mut_ptr(), rx_pkts.len() as u16) as usize
        }
    }

    fn tx_burst(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr]) -> usize {
        unsafe {
            if rx_pkts.is_empty() {
                _rte_eth_tx_burst(*self, queue_id, ptr::null_mut(), 0) as usize
            } else {
                _rte_eth_tx_burst(*self,
                                  queue_id,
                                  rx_pkts.as_mut_ptr(),
                                  rx_pkts.len() as u16) as usize
            }
        }
    }

    fn rx_burst_ex(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr], packets: u16) -> u16 {
        unsafe {
            _rte_eth_rx_burst(*self, queue_id, rx_pkts.as_mut_ptr(), packets)
        }
    }

    fn tx_burst_ex(&self, queue_id: QueueId, rx_pkts: &mut [mbuf::RawMbufPtr], packets: u16) -> u16 {
        unsafe {
            _rte_eth_tx_burst(*self, queue_id, rx_pkts.as_mut_ptr(), packets)
        }
    }


    fn set_vf_rxmode(&self, vf: u16, rx_mode: EthVmdqRxMode, on: bool) -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_set_vf_rxmode(*self, vf, rx_mode.bits, bool_value!(on))
        }; ok => { self })
    }

    fn set_vf_tx(&self, vf: u16, on: bool) -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_set_vf_tx(*self, vf, bool_value!(on))
        }; ok => { self })
    }

    fn set_vf_rx(&self, vf: u16, on: bool) -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_set_vf_rx(*self, vf, bool_value!(on))
        }; ok => { self })
    }

    fn vlan_offload(&self) -> Result<EthVlanOffloadMode> {
        let mode = unsafe { ffi::rte_eth_dev_get_vlan_offload(*self) };

        rte_check!(mode; ok => { EthVlanOffloadMode::from_bits_truncate(mode) })
    }

    fn set_vlan_offload(&self, mode: EthVlanOffloadMode) -> Result<&Self> {
        rte_check!(unsafe {
            ffi::rte_eth_dev_set_vlan_offload(*self, mode.bits)
        }; ok => { self })
    }
}

pub trait EthDeviceInfo {
    /// Device Driver name.
    fn driver_name(&self) -> &str;

    fn pci_dev(&self) -> Option<&mut pci::RawPciDevice>;
}

pub type RawEthDeviceInfo = ffi::Struct_rte_eth_dev_info;

impl EthDeviceInfo for RawEthDeviceInfo {
    #[inline]
    fn driver_name(&self) -> &str {
        unsafe { CStr::from_ptr(self.driver_name).to_str().unwrap() }
    }

    #[inline]
    fn pci_dev(&self) -> Option<&mut pci::RawPciDevice> {
        self.pci_dev.as_mut_ref()
    }
}

pub trait EthDeviceStats {}

pub type RawEthDeviceStats = ffi::Struct_rte_eth_stats;

impl EthDeviceStats for RawEthDeviceStats {}

bitflags! {
    /// Definitions used for VMDQ pool rx mode setting
    pub flags EthVmdqRxMode : u16 {
        /// accept untagged packets.
        const ETH_VMDQ_ACCEPT_UNTAG     = 0x0001,
        /// accept packets in multicast table .
        const ETH_VMDQ_ACCEPT_HASH_MC   = 0x0002,
        /// accept packets in unicast table.
        const ETH_VMDQ_ACCEPT_HASH_UC   = 0x0004,
        /// accept broadcast packets.
        const ETH_VMDQ_ACCEPT_BROADCAST = 0x0008,
        /// multicast promiscuous.
        const ETH_VMDQ_ACCEPT_MULTICAST = 0x0010,
    }
}

/// A set of values to identify what method is to be used to route packets to multiple queues.
bitflags! {
    pub flags EthRxMultiQueueMode: u32 {
        const ETH_MQ_RX_RSS_FLAG    = 0x1,
        const ETH_MQ_RX_DCB_FLAG    = 0x2,
        const ETH_MQ_RX_VMDQ_FLAG   = 0x4,
    }
}

bitflags! {
    /// Definitions used for VLAN Offload functionality
    pub flags EthVlanOffloadMode: i32 {
        /// VLAN Strip  On/Off
        const ETH_VLAN_STRIP_OFFLOAD  = 0x0001,
        /// VLAN Filter On/Off
        const ETH_VLAN_FILTER_OFFLOAD = 0x0002,
        /// VLAN Extend On/Off
        const ETH_VLAN_EXTEND_OFFLOAD = 0x0004,

        /// VLAN Strip  setting mask
        const ETH_VLAN_STRIP_MASK     = 0x0001,
        /// VLAN Filter  setting mask
        const ETH_VLAN_FILTER_MASK    = 0x0002,
        /// VLAN Extend  setting mask
        const ETH_VLAN_EXTEND_MASK    = 0x0004,
        /// VLAN ID is in lower 12 bits
        const ETH_VLAN_ID_MAX         = 0x0FFF,
    }
}

/// A structure used to configure the RX features of an Ethernet port.
pub struct EthRxMode {
    /// The multi-queue packet distribution mode to be used, e.g. RSS.
    pub mq_mode: EthRxMultiQueueMode,
    /// Header Split enable.
    pub split_hdr_size: u16,
    /// IP/UDP/TCP checksum offload enable.
    pub hw_ip_checksum: bool,
    /// VLAN filter enable.
    pub hw_vlan_filter: bool,
    /// VLAN strip enable.
    pub hw_vlan_strip: bool,
    /// Extended VLAN enable.
    pub hw_vlan_extend: bool,
    /// Jumbo Frame Receipt enable.
    pub max_rx_pkt_len: u32,
    /// Enable CRC stripping by hardware.
    pub hw_strip_crc: bool,
    /// Enable scatter packets rx handler
    pub enable_scatter: bool,
    /// Enable LRO
    pub enable_lro: bool,
}

impl Default for EthRxMode {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/**
 * A set of values to identify what method is to be used to transmit
 * packets using multi-TCs.
 */
pub type EthTxMultiQueueMode = ffi::Enum_rte_eth_tx_mq_mode;

pub struct EthTxMode {
    /// TX multi-queues mode.
    pub mq_mode: EthTxMultiQueueMode,
    /// If set, reject sending out tagged pkts
    pub hw_vlan_reject_tagged: bool,
    /// If set, reject sending out untagged pkts
    pub hw_vlan_reject_untagged: bool,
    /// If set, enable port based VLAN insertion
    pub hw_vlan_insert_pvid: bool,
}

impl Default for EthTxMode {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// The RSS offload types are defined based on flow types which are defined
/// in rte_eth_ctrl.h. Different NIC hardwares may support different RSS offload
/// types. The supported flow types or RSS offload types can be queried by
/// rte_eth_dev_info_get().
bitflags! {
    pub flags RssHashFunc: u64 {
        const ETH_RSS_IPV4               = 1 << ::ffi::consts::RTE_ETH_FLOW_IPV4,
        const ETH_RSS_FRAG_IPV4          = 1 << ::ffi::consts::RTE_ETH_FLOW_FRAG_IPV4,
        const ETH_RSS_NONFRAG_IPV4_TCP   = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV4_TCP,
        const ETH_RSS_NONFRAG_IPV4_UDP   = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV4_UDP,
        const ETH_RSS_NONFRAG_IPV4_SCTP  = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV4_SCTP,
        const ETH_RSS_NONFRAG_IPV4_OTHER = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV4_OTHER,
        const ETH_RSS_IPV6               = 1 << ::ffi::consts::RTE_ETH_FLOW_IPV6,
        const ETH_RSS_FRAG_IPV6          = 1 << ::ffi::consts::RTE_ETH_FLOW_FRAG_IPV6,
        const ETH_RSS_NONFRAG_IPV6_TCP   = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV6_TCP,
        const ETH_RSS_NONFRAG_IPV6_UDP   = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV6_UDP,
        const ETH_RSS_NONFRAG_IPV6_SCTP  = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV6_SCTP,
        const ETH_RSS_NONFRAG_IPV6_OTHER = 1 << ::ffi::consts::RTE_ETH_FLOW_NONFRAG_IPV6_OTHER,
        const ETH_RSS_L2_PAYLOAD         = 1 << ::ffi::consts::RTE_ETH_FLOW_L2_PAYLOAD,
        const ETH_RSS_IPV6_EX            = 1 << ::ffi::consts::RTE_ETH_FLOW_IPV6_EX,
        const ETH_RSS_IPV6_TCP_EX        = 1 << ::ffi::consts::RTE_ETH_FLOW_IPV6_TCP_EX,
        const ETH_RSS_IPV6_UDP_EX        = 1 << ::ffi::consts::RTE_ETH_FLOW_IPV6_UDP_EX,

        const ETH_RSS_IP =
            ETH_RSS_IPV4.bits |
            ETH_RSS_FRAG_IPV4.bits |
            ETH_RSS_NONFRAG_IPV4_OTHER.bits |
            ETH_RSS_IPV6.bits |
            ETH_RSS_FRAG_IPV6.bits |
            ETH_RSS_NONFRAG_IPV6_OTHER.bits |
            ETH_RSS_IPV6_EX.bits,

        const ETH_RSS_UDP =
            ETH_RSS_NONFRAG_IPV4_UDP.bits |
            ETH_RSS_NONFRAG_IPV6_UDP.bits |
            ETH_RSS_IPV6_UDP_EX.bits,

        const ETH_RSS_TCP =
            ETH_RSS_NONFRAG_IPV4_TCP.bits |
            ETH_RSS_NONFRAG_IPV6_TCP.bits |
            ETH_RSS_IPV6_TCP_EX.bits,

        const ETH_RSS_SCTP =
            ETH_RSS_NONFRAG_IPV4_SCTP.bits |
            ETH_RSS_NONFRAG_IPV6_SCTP.bits,

        /**< Mask of valid RSS hash protocols */
        const ETH_RSS_PROTO_MASK =
            ETH_RSS_IPV4.bits |
            ETH_RSS_FRAG_IPV4.bits |
            ETH_RSS_NONFRAG_IPV4_TCP.bits |
            ETH_RSS_NONFRAG_IPV4_UDP.bits |
            ETH_RSS_NONFRAG_IPV4_SCTP.bits |
            ETH_RSS_NONFRAG_IPV4_OTHER.bits |
            ETH_RSS_IPV6.bits |
            ETH_RSS_FRAG_IPV6.bits |
            ETH_RSS_NONFRAG_IPV6_TCP.bits |
            ETH_RSS_NONFRAG_IPV6_UDP.bits |
            ETH_RSS_NONFRAG_IPV6_SCTP.bits |
            ETH_RSS_NONFRAG_IPV6_OTHER.bits |
            ETH_RSS_L2_PAYLOAD.bits |
            ETH_RSS_IPV6_EX.bits |
            ETH_RSS_IPV6_TCP_EX.bits |
            ETH_RSS_IPV6_UDP_EX.bits,
    }
}

pub struct EthRssConf {
    pub key: Option<[u8; 40]>,
    pub hash: RssHashFunc,
}

impl Default for EthRssConf {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[derive(Default)]
pub struct RxAdvConf {
    /// Port RSS configuration
    pub rss_conf: Option<EthRssConf>,
    pub vmdq_dcb_conf: Option<ffi::Struct_rte_eth_vmdq_dcb_conf>,
    pub dcb_rx_conf: Option<ffi::Struct_rte_eth_dcb_rx_conf>,
    pub vmdq_rx_conf: Option<ffi::Struct_rte_eth_vmdq_rx_conf>,
}

pub enum TxAdvConf {

}

/// Device supported speeds bitmap flags
bitflags! {
    pub flags LinkSpeed: u32 {
        /**< Autonegotiate (all speeds) */
        const ETH_LINK_SPEED_AUTONEG  = 0 <<  0,
        /**< Disable autoneg (fixed speed) */
        const ETH_LINK_SPEED_FIXED    = 1 <<  0,
        /**<  10 Mbps half-duplex */
        const ETH_LINK_SPEED_10M_HD   = 1 <<  1,
         /**<  10 Mbps full-duplex */
        const ETH_LINK_SPEED_10M      = 1 <<  2,
        /**< 100 Mbps half-duplex */
        const ETH_LINK_SPEED_100M_HD  = 1 <<  3,
        /**< 100 Mbps full-duplex */
        const ETH_LINK_SPEED_100M     = 1 <<  4,
        const ETH_LINK_SPEED_1G       = 1 <<  5,
        const ETH_LINK_SPEED_2_5G     = 1 <<  6,
        const ETH_LINK_SPEED_5G       = 1 <<  7,
        const ETH_LINK_SPEED_10G      = 1 <<  8,
        const ETH_LINK_SPEED_20G      = 1 <<  9,
        const ETH_LINK_SPEED_25G      = 1 << 10,
        const ETH_LINK_SPEED_40G      = 1 << 11,
        const ETH_LINK_SPEED_50G      = 1 << 12,
        const ETH_LINK_SPEED_56G      = 1 << 13,
        const ETH_LINK_SPEED_100G     = 1 << 14,
    }
}

impl Default for LinkSpeed {
    fn default() -> Self {
        ETH_LINK_SPEED_AUTONEG
    }
}

#[derive(Default)]
pub struct EthConf {
    /// bitmap of ETH_LINK_SPEED_XXX of speeds to be used.
    ///
    /// ETH_LINK_SPEED_FIXED disables link autonegotiation, and a unique speed shall be set.
    /// Otherwise, the bitmap defines the set of speeds to be advertised.
    /// If the special value ETH_LINK_SPEED_AUTONEG (0) is used,
    /// all speeds supported are advertised.
    pub link_speeds: LinkSpeed,
    /// Port RX configuration.
    pub rxmode: Option<EthRxMode>,
    /// Port TX configuration.
    pub txmode: Option<EthTxMode>,
    /// Loopback operation mode.
    ///
    /// By default the value is 0, meaning the loopback mode is disabled.
    /// Read the datasheet of given ethernet controller for details.
    /// The possible values of this field are defined in implementation of each driver.
    pub lpbk_mode: u32,
    /// Port RX filtering configuration (union).
    pub rx_adv_conf: Option<RxAdvConf>,
    /// Port TX DCB configuration (union).
    pub tx_adv_conf: Option<TxAdvConf>,
    /// Currently,Priority Flow Control(PFC) are supported,
    /// if DCB with PFC is needed, and the variable must be set ETH_DCB_PFC_SUPPORT.
    pub dcb_capability_en: u32,
    pub fdir_conf: Option<ffi::Struct_rte_fdir_conf>,
    pub intr_conf: Option<ffi::Struct_rte_intr_conf>,
}

pub type RawEthConfPtr = *const ffi::Struct_rte_eth_conf;

pub struct RawEthConf(RawEthConfPtr);

impl RawEthConf {
    fn as_raw(&self) -> RawEthConfPtr {
        self.0
    }
}

impl Drop for RawEthConf {
    fn drop(&mut self) {
        unsafe { _rte_eth_conf_free(self.0) }
    }
}

impl<'a> From<&'a EthConf> for RawEthConf {
    fn from(c: &EthConf) -> Self {
        unsafe {
            let conf = _rte_eth_conf_new();

            if let Some(ref rxmode) = c.rxmode {
                _rte_eth_conf_set_rx_mode(conf,
                                          rxmode.mq_mode.bits,
                                          rxmode.split_hdr_size,
                                          rxmode.hw_ip_checksum as u8,
                                          rxmode.hw_vlan_filter as u8,
                                          rxmode.hw_vlan_strip as u8,
                                          rxmode.hw_vlan_extend as u8,
                                          rxmode.max_rx_pkt_len,
                                          rxmode.hw_strip_crc as u8,
                                          rxmode.enable_scatter as u8,
                                          rxmode.enable_lro as u8);
            }

            if let Some(ref txmode) = c.txmode {
                _rte_eth_conf_set_tx_mode(conf,
                                          txmode.mq_mode as u32,
                                          txmode.hw_vlan_reject_tagged as u8,
                                          txmode.hw_vlan_reject_untagged as u8,
                                          txmode.hw_vlan_insert_pvid as u8);
            }

            if let Some(ref adv_conf) = c.rx_adv_conf {
                if let Some(ref rss_conf) = adv_conf.rss_conf {
                    let (rss_key, rss_key_len) = rss_conf.key
                        .map_or_else(|| (ptr::null(), 0), |key| (key.as_ptr(), key.len() as u8));

                    _rte_eth_conf_set_rss_conf(conf, rss_key, rss_key_len, rss_conf.hash.bits);
                }
            }

            RawEthConf(conf)
        }
    }
}

pub type RawTxBuffer = ffi::Struct_rte_eth_dev_tx_buffer;
pub type RawTxBufferPtr = *mut ffi::Struct_rte_eth_dev_tx_buffer;

pub type TxBufferErrorCallback<T> = fn(unsent: *mut *mut ffi::Struct_rte_mbuf,
                                       count: u16,
                                       userdata: &T);

pub trait TxBuffer {
    fn free(&mut self);

    /// Configure a callback for buffered packets which cannot be sent
    fn set_err_callback<T>(&mut self,
                           callback: Option<TxBufferErrorCallback<T>>,
                           userdata: Option<&T>)
                           -> Result<&mut Self>;

    /// Silently dropping unsent buffered packets.
    fn drop_err_packets(&mut self) -> Result<&mut Self>;

    /// Tracking unsent buffered packets.
    fn count_err_packets(&mut self) -> Result<&mut Self>;
}

/// Initialize default values for buffered transmitting
pub fn alloc_buffer(size: usize, socket_id: i32) -> Result<RawTxBufferPtr> {
    unsafe {
        let p = malloc::zmalloc_socket("tx_buffer",
                                       _rte_eth_tx_buffer_size(size),
                                       0,
                                       socket_id) as RawTxBufferPtr;

        if p.is_null() {
            Err(Error::OsError(libc::ENOMEM))
        } else {
            let ret = ffi::rte_eth_tx_buffer_init(p, size as u16);

            if ret != 0 {
                Err(Error::OsError(ret))
            } else {
                Ok(p)
            }
        }
    }
}

impl TxBuffer for RawTxBuffer {
    fn free(&mut self) {
        malloc::free(self as RawTxBufferPtr as *mut c_void);
    }

    fn set_err_callback<T>(&mut self,
                           callback: Option<TxBufferErrorCallback<T>>,
                           userdata: Option<&T>)
                           -> Result<&mut Self> {
        rte_check!(unsafe {
            ffi::rte_eth_tx_buffer_set_err_callback(self,
                                                    mem::transmute(callback),
                                                    mem::transmute(userdata))
        }; ok => { self })
    }

    fn drop_err_packets(&mut self) -> Result<&mut Self> {
        rte_check!(unsafe {
            ffi::rte_eth_tx_buffer_set_err_callback(self,
                                                    Some(ffi::rte_eth_tx_buffer_drop_callback),
                                                    ptr::null_mut())
        }; ok => { self })
    }

    fn count_err_packets(&mut self) -> Result<&mut Self> {
        rte_check!(unsafe {
            ffi::rte_eth_tx_buffer_set_err_callback(self,
                                                    Some(ffi::rte_eth_tx_buffer_count_callback),
                                                    ptr::null_mut())
        }; ok => { self })
    }
}

extern "C" {
    fn _rte_eth_rx_burst(port_id: libc::uint8_t,
                         queue_id: libc::uint16_t,
                         rx_pkts: *mut mbuf::RawMbufPtr,
                         nb_pkts: libc::uint16_t)
                         -> libc::uint16_t;

    fn _rte_eth_tx_burst(port_id: libc::uint8_t,
                         queue_id: libc::uint16_t,
                         tx_pkts: *mut mbuf::RawMbufPtr,
                         nb_pkts: libc::uint16_t)
                         -> libc::uint16_t;

    fn _rte_eth_conf_new() -> RawEthConfPtr;

    fn _rte_eth_conf_free(conf: RawEthConfPtr);

    fn _rte_eth_conf_set_rx_mode(conf: RawEthConfPtr,
                                 mq_mode: libc::uint32_t,
                                 split_hdr_size: libc::uint16_t,
                                 hw_ip_checksum: libc::uint8_t,
                                 hw_vlan_filter: libc::uint8_t,
                                 hw_vlan_strip: libc::uint8_t,
                                 hw_vlan_extend: libc::uint8_t,
                                 max_rx_pkt_len: libc::uint32_t,
                                 hw_strip_crc: libc::uint8_t,
                                 enable_scatter: libc::uint8_t,
                                 enable_lro: libc::uint8_t);

    fn _rte_eth_conf_set_tx_mode(conf: RawEthConfPtr,
                                 mq_mode: libc::uint32_t,
                                 hw_vlan_reject_tagged: libc::uint8_t,
                                 hw_vlan_reject_untagged: libc::uint8_t,
                                 hw_vlan_insert_pvid: libc::uint8_t);

    fn _rte_eth_conf_set_rss_conf(conf: RawEthConfPtr,
                                  rss_key: *const libc::uint8_t,
                                  rss_key_len: libc::uint8_t,
                                  rss_hf: libc::uint64_t);

    fn _rte_eth_tx_buffer_size(size: libc::size_t) -> libc::size_t;
}
