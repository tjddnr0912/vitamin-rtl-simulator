// Testbench for verilog-ethernet eth_mac_1g (GMII loopback)
// Pushes N synthetic frames through the MAC TX path, loops GMII back into the
// RX path, and accumulates every RX-side cycle into a rotate-xor DIGEST.
`resetall
`timescale 1ns / 1ps
`default_nettype none

module tb;

integer N;
integer wd_cycles;
integer frame;
integer len;
integer b;
integer idle;
integer rx_frames;
integer rx_bytes;

reg [31:0] s;
reg [63:0] digest;
reg [63:0] cyc;

reg clk;
reg rst;

reg  [7:0] tx_axis_tdata;
reg        tx_axis_tvalid;
wire       tx_axis_tready;
reg        tx_axis_tlast;
reg        tx_axis_tuser;

wire [7:0] rx_axis_tdata;
wire       rx_axis_tvalid;
wire       rx_axis_tlast;
wire       rx_axis_tuser;

wire [7:0] gmii_txd;
wire       gmii_tx_en;
wire       gmii_tx_er;

reg  [7:0] gmii_rxd;
reg        gmii_rx_dv;
reg        gmii_rx_er;

wire       tx_start_packet;
wire       tx_error_underflow;
wire       rx_start_packet;
wire       rx_error_bad_frame;
wire       rx_error_bad_fcs;

wire [95:0] ptp_zero = 96'd0;

// ---------------- clock ----------------
initial begin
    clk = 1'b0;
end
always #4 clk = ~clk;

// ---------------- GMII loopback (registered) ----------------
always @(posedge clk) begin
    gmii_rxd   <= gmii_txd;
    gmii_rx_dv <= gmii_tx_en;
    gmii_rx_er <= gmii_tx_er;
end

// ---------------- DUT ----------------
eth_mac_1g #(
    .DATA_WIDTH(8),
    .ENABLE_PADDING(1),
    .MIN_FRAME_LENGTH(64),
    .PTP_TS_ENABLE(0),
    .PFC_ENABLE(0),
    .PAUSE_ENABLE(0)
)
uut (
    .rx_clk(clk),
    .rx_rst(rst),
    .tx_clk(clk),
    .tx_rst(rst),

    .tx_axis_tdata(tx_axis_tdata),
    .tx_axis_tvalid(tx_axis_tvalid),
    .tx_axis_tready(tx_axis_tready),
    .tx_axis_tlast(tx_axis_tlast),
    .tx_axis_tuser(tx_axis_tuser),

    .rx_axis_tdata(rx_axis_tdata),
    .rx_axis_tvalid(rx_axis_tvalid),
    .rx_axis_tlast(rx_axis_tlast),
    .rx_axis_tuser(rx_axis_tuser),

    .gmii_rxd(gmii_rxd),
    .gmii_rx_dv(gmii_rx_dv),
    .gmii_rx_er(gmii_rx_er),
    .gmii_txd(gmii_txd),
    .gmii_tx_en(gmii_tx_en),
    .gmii_tx_er(gmii_tx_er),

    .tx_ptp_ts(ptp_zero),
    .rx_ptp_ts(ptp_zero),

    .tx_lfc_req(1'b0),
    .tx_lfc_resend(1'b0),
    .rx_lfc_en(1'b0),
    .rx_lfc_ack(1'b0),

    .tx_pfc_req(8'd0),
    .tx_pfc_resend(1'b0),
    .rx_pfc_en(8'd0),
    .rx_pfc_ack(8'd0),

    .tx_lfc_pause_en(1'b0),
    .tx_pause_req(1'b0),

    .rx_clk_enable(1'b1),
    .tx_clk_enable(1'b1),
    .rx_mii_select(1'b0),
    .tx_mii_select(1'b0),

    .tx_start_packet(tx_start_packet),
    .tx_error_underflow(tx_error_underflow),
    .rx_start_packet(rx_start_packet),
    .rx_error_bad_frame(rx_error_bad_frame),
    .rx_error_bad_fcs(rx_error_bad_fcs),

    .cfg_ifg(8'd12),
    .cfg_tx_enable(1'b1),
    .cfg_rx_enable(1'b1),
    .cfg_mcf_rx_eth_dst_mcast(48'd0),
    .cfg_mcf_rx_check_eth_dst_mcast(1'b0),
    .cfg_mcf_rx_eth_dst_ucast(48'd0),
    .cfg_mcf_rx_check_eth_dst_ucast(1'b0),
    .cfg_mcf_rx_eth_src(48'd0),
    .cfg_mcf_rx_check_eth_src(1'b0),
    .cfg_mcf_rx_eth_type(16'd0),
    .cfg_mcf_rx_opcode_lfc(16'd0),
    .cfg_mcf_rx_check_opcode_lfc(1'b0),
    .cfg_mcf_rx_opcode_pfc(16'd0),
    .cfg_mcf_rx_check_opcode_pfc(1'b0),
    .cfg_mcf_rx_forward(1'b0),
    .cfg_mcf_rx_enable(1'b0),
    .cfg_tx_lfc_eth_dst(48'd0),
    .cfg_tx_lfc_eth_src(48'd0),
    .cfg_tx_lfc_eth_type(16'd0),
    .cfg_tx_lfc_opcode(16'd0),
    .cfg_tx_lfc_en(1'b0),
    .cfg_tx_lfc_quanta(16'd0),
    .cfg_tx_lfc_refresh(16'd0),
    .cfg_tx_pfc_eth_dst(48'd0),
    .cfg_tx_pfc_eth_src(48'd0),
    .cfg_tx_pfc_eth_type(16'd0),
    .cfg_tx_pfc_opcode(16'd0),
    .cfg_tx_pfc_en(1'b0),
    .cfg_tx_pfc_quanta(128'd0),
    .cfg_tx_pfc_refresh(128'd0),
    .cfg_rx_lfc_opcode(16'd0),
    .cfg_rx_lfc_en(1'b0),
    .cfg_rx_pfc_opcode(16'd0),
    .cfg_rx_pfc_en(1'b0)
);

// ---------------- digest accumulator ----------------
always @(posedge clk) begin
    cyc <= cyc + 64'd1;
    if (rst) begin
        digest <= 64'h0123456789abcdef;
    end else begin
        digest <= {digest[62:0], digest[63]}
                ^ (rx_axis_tvalid ? {54'd0, rx_axis_tuser, rx_axis_tlast, rx_axis_tdata} : 64'd0)
                ^ {59'd0, rx_error_bad_fcs, rx_error_bad_frame, rx_start_packet,
                          tx_error_underflow, tx_start_packet};
        if (rx_axis_tvalid) begin
            rx_bytes = rx_bytes + 1;
            if (rx_axis_tlast) rx_frames = rx_frames + 1;
        end
    end
    // watchdog
    if (cyc > wd_cycles) begin
        $display("WATCHDOG");
        $display("DIGEST=%016x", digest);
        $finish;
    end
end

function [31:0] nxt;
    input [31:0] v;
    begin
        nxt = {v[30:0], v[31] ^ v[21] ^ v[1] ^ v[0]};
    end
endfunction

// ---------------- stimulus ----------------
initial begin
    N = 500;
    if ($value$plusargs("N=%d", N)) ;
    wd_cycles = N * 800 + 200000;

    rst = 1'b1;
    tx_axis_tdata  = 8'd0;
    tx_axis_tvalid = 1'b0;
    tx_axis_tlast  = 1'b0;
    tx_axis_tuser  = 1'b0;
    gmii_rxd       = 8'd0;
    gmii_rx_dv     = 1'b0;
    gmii_rx_er     = 1'b0;
    digest         = 64'h0123456789abcdef;
    cyc            = 64'd0;
    rx_frames      = 0;
    rx_bytes       = 0;
    s              = 32'h1234_5678;

    repeat (16) @(posedge clk);
    rst <= 1'b0;
    repeat (4) @(posedge clk);

    for (frame = 0; frame < N; frame = frame + 1) begin
        s   = nxt(s);
        len = 20 + (s % 260);
        for (b = 0; b < len; b = b + 1) begin
            s = nxt(s);
            tx_axis_tdata  <= s[7:0];
            tx_axis_tvalid <= 1'b1;
            tx_axis_tlast  <= (b == len - 1);
            tx_axis_tuser  <= 1'b0;
            @(posedge clk);
            while (tx_axis_tready !== 1'b1) @(posedge clk);
        end
        tx_axis_tvalid <= 1'b0;
        tx_axis_tlast  <= 1'b0;
        @(posedge clk);
    end

    // drain
    idle = 0;
    while (idle < 4000) begin
        @(posedge clk);
        if (rx_axis_tvalid) idle = 0;
        else idle = idle + 1;
    end

    $display("RX_FRAMES=%0d RX_BYTES=%0d", rx_frames, rx_bytes);
    $display("DIGEST=%016x", digest ^ {rx_frames[31:0], rx_bytes[31:0]});
    $finish;
end

endmodule

`resetall
