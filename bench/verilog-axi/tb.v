// Benchmark testbench for alexforencich/verilog-axi (MIT).
// 2x2 axi_crossbar feeding two axi_ram instances, driven by two synthetic
// AXI4 master BFMs that write then read back bursts and accumulate a digest.
`timescale 1ns / 1ps
`default_nettype wire

module axi_master_bfm #(
    parameter DATA_WIDTH = 32,
    parameter ADDR_WIDTH = 32,
    parameter STRB_WIDTH = (DATA_WIDTH/8),
    parameter ID_WIDTH   = 4,
    parameter MID        = 0,
    parameter SEED       = 32'h1234_5678
)
(
    input  wire                   clk,
    input  wire                   rst,
    input  wire [31:0]            num_ops,

    output reg  [ID_WIDTH-1:0]    awid,
    output reg  [ADDR_WIDTH-1:0]  awaddr,
    output reg  [7:0]             awlen,
    output reg  [2:0]             awsize,
    output reg  [1:0]             awburst,
    output reg                    awvalid,
    input  wire                   awready,
    output reg  [DATA_WIDTH-1:0]  wdata,
    output reg  [STRB_WIDTH-1:0]  wstrb,
    output reg                    wlast,
    output reg                    wvalid,
    input  wire                   wready,
    input  wire [ID_WIDTH-1:0]    bid,
    input  wire [1:0]             bresp,
    input  wire                   bvalid,
    output reg                    bready,
    output reg  [ID_WIDTH-1:0]    arid,
    output reg  [ADDR_WIDTH-1:0]  araddr,
    output reg  [7:0]             arlen,
    output reg  [2:0]             arsize,
    output reg  [1:0]             arburst,
    output reg                    arvalid,
    input  wire                   arready,
    input  wire [ID_WIDTH-1:0]    rid,
    input  wire [DATA_WIDTH-1:0]  rdata,
    input  wire [1:0]             rresp,
    input  wire                   rlast,
    input  wire                   rvalid,
    output reg                    rready,

    output reg  [63:0]            digest,
    output reg                    done
);

reg [31:0] lfsr;
reg [31:0] op;
reg [31:0] addr;
reg [7:0]  blen;
reg [31:0] k;
reg [31:0] wd;
reg        got_last;
integer    i;

function [31:0] lfsr_next;
    input [31:0] s;
    begin
        lfsr_next = {s[30:0], s[31] ^ s[21] ^ s[1] ^ s[0]};
    end
endfunction

initial begin
    awid    = {ID_WIDTH{1'b0}};
    awaddr  = {ADDR_WIDTH{1'b0}};
    awlen   = 8'd0;
    awsize  = 3'd2;
    awburst = 2'b01;
    awvalid = 1'b0;
    wdata   = {DATA_WIDTH{1'b0}};
    wstrb   = {STRB_WIDTH{1'b1}};
    wlast   = 1'b0;
    wvalid  = 1'b0;
    bready  = 1'b1;
    arid    = {ID_WIDTH{1'b0}};
    araddr  = {ADDR_WIDTH{1'b0}};
    arlen   = 8'd0;
    arsize  = 3'd2;
    arburst = 2'b01;
    arvalid = 1'b0;
    rready  = 1'b0;
    digest  = 64'd0;
    done    = 1'b0;
    lfsr    = SEED;
    blen    = 8'd0;
    addr    = 32'd0;
    k       = 32'd0;
    wd      = 32'd0;
    got_last = 1'b0;

    @(negedge rst);
    repeat (4) @(posedge clk);

    for (op = 32'd0; op < num_ops; op = op + 32'd1) begin
        for (i = 0; i < 7; i = i + 1) lfsr = lfsr_next(lfsr);

        blen = {5'd0, lfsr[2:0]};                       // 1..8 beats
        // 32-byte-aligned start, private 4 KiB window per master, 2 slaves
        addr = (lfsr[3] ? 32'h0100_0000 : 32'h0000_0000)
             | (MID << 13)
             | ({22'd0, lfsr[13:7], 3'd0} << 2);

        // ---- write address ----
        awid    <= lfsr[17:14];
        awaddr  <= addr;
        awlen   <= blen;
        awvalid <= 1'b1;
        @(posedge clk);
        while (!awready) @(posedge clk);
        awvalid <= 1'b0;

        // ---- write data ----
        for (k = 32'd0; k <= {24'd0, blen}; k = k + 32'd1) begin
            wd = (addr + (k * 32'h9E37_79B9)) ^ (32'h5A5A_0000 + MID);
            wdata  <= wd;
            wlast  <= (k == {24'd0, blen});
            wvalid <= 1'b1;
            @(posedge clk);
            while (!wready) @(posedge clk);
        end
        wvalid <= 1'b0;
        wlast  <= 1'b0;

        // ---- write response ----
        @(posedge clk);
        while (!bvalid) @(posedge clk);
        digest = digest + {58'd0, bresp, bid[3:0]};

        // ---- read address ----
        arid    <= lfsr[21:18];
        araddr  <= addr;
        arlen   <= blen;
        arvalid <= 1'b1;
        @(posedge clk);
        while (!arready) @(posedge clk);
        arvalid <= 1'b0;

        // ---- read data ----
        rready   <= 1'b1;
        got_last = 1'b0;
        k        = 32'd0;
        while (!got_last) begin
            @(posedge clk);
            if (rvalid) begin
                digest   = digest + ({32'd0, rdata} * ({32'd0, k} + 64'd1))
                                  + {62'd0, rresp} + {60'd0, rid[3:0]};
                k        = k + 32'd1;
                got_last = rlast;
            end
        end
        rready <= 1'b0;
    end

    done = 1'b1;
end

endmodule


module tb;

localparam S_COUNT        = 2;
localparam M_COUNT        = 2;
localparam DATA_WIDTH     = 32;
localparam ADDR_WIDTH     = 32;
localparam STRB_WIDTH     = (DATA_WIDTH/8);
localparam S_ID_WIDTH     = 4;
localparam M_ID_WIDTH     = 5;   // S_ID_WIDTH + $clog2(S_COUNT)
localparam RAM_ADDR_WIDTH = 16;

reg clk = 1'b0;
reg rst = 1'b1;
always #5 clk = ~clk;

reg [31:0] num_ops;

// ---- per-master slave-side signals ----
wire [S_ID_WIDTH-1:0]  s_awid   [0:S_COUNT-1];
wire [ADDR_WIDTH-1:0]  s_awaddr [0:S_COUNT-1];
wire [7:0]             s_awlen  [0:S_COUNT-1];
wire [2:0]             s_awsize [0:S_COUNT-1];
wire [1:0]             s_awburst[0:S_COUNT-1];
wire [S_COUNT-1:0]     s_awvalid;
wire [S_COUNT-1:0]     s_awready;
wire [DATA_WIDTH-1:0]  s_wdata  [0:S_COUNT-1];
wire [STRB_WIDTH-1:0]  s_wstrb  [0:S_COUNT-1];
wire [S_COUNT-1:0]     s_wlast;
wire [S_COUNT-1:0]     s_wvalid;
wire [S_COUNT-1:0]     s_wready;
wire [S_COUNT-1:0]     s_bvalid;
wire [S_COUNT-1:0]     s_bready;
wire [S_ID_WIDTH-1:0]  s_arid   [0:S_COUNT-1];
wire [ADDR_WIDTH-1:0]  s_araddr [0:S_COUNT-1];
wire [7:0]             s_arlen  [0:S_COUNT-1];
wire [2:0]             s_arsize [0:S_COUNT-1];
wire [1:0]             s_arburst[0:S_COUNT-1];
wire [S_COUNT-1:0]     s_arvalid;
wire [S_COUNT-1:0]     s_arready;
wire [S_COUNT-1:0]     s_rlast;
wire [S_COUNT-1:0]     s_rvalid;
wire [S_COUNT-1:0]     s_rready;

wire [S_COUNT*S_ID_WIDTH-1:0] s_axi_bid;
wire [S_COUNT*2-1:0]          s_axi_bresp;
wire [S_COUNT*S_ID_WIDTH-1:0] s_axi_rid;
wire [S_COUNT*DATA_WIDTH-1:0] s_axi_rdata;
wire [S_COUNT*2-1:0]          s_axi_rresp;

wire [63:0] m_digest [0:S_COUNT-1];
wire [S_COUNT-1:0] m_done;

genvar gi;
generate
    for (gi = 0; gi < S_COUNT; gi = gi + 1) begin : mst
        axi_master_bfm #(
            .DATA_WIDTH(DATA_WIDTH),
            .ADDR_WIDTH(ADDR_WIDTH),
            .STRB_WIDTH(STRB_WIDTH),
            .ID_WIDTH(S_ID_WIDTH),
            .MID(gi),
            .SEED(32'h1234_5678 + (gi * 32'h0BAD_F00D))
        ) bfm (
            .clk(clk), .rst(rst), .num_ops(num_ops),
            .awid(s_awid[gi]), .awaddr(s_awaddr[gi]), .awlen(s_awlen[gi]),
            .awsize(s_awsize[gi]), .awburst(s_awburst[gi]),
            .awvalid(s_awvalid[gi]), .awready(s_awready[gi]),
            .wdata(s_wdata[gi]), .wstrb(s_wstrb[gi]), .wlast(s_wlast[gi]),
            .wvalid(s_wvalid[gi]), .wready(s_wready[gi]),
            .bid(s_axi_bid[gi*S_ID_WIDTH +: S_ID_WIDTH]),
            .bresp(s_axi_bresp[gi*2 +: 2]),
            .bvalid(s_bvalid[gi]), .bready(s_bready[gi]),
            .arid(s_arid[gi]), .araddr(s_araddr[gi]), .arlen(s_arlen[gi]),
            .arsize(s_arsize[gi]), .arburst(s_arburst[gi]),
            .arvalid(s_arvalid[gi]), .arready(s_arready[gi]),
            .rid(s_axi_rid[gi*S_ID_WIDTH +: S_ID_WIDTH]),
            .rdata(s_axi_rdata[gi*DATA_WIDTH +: DATA_WIDTH]),
            .rresp(s_axi_rresp[gi*2 +: 2]),
            .rlast(s_rlast[gi]), .rvalid(s_rvalid[gi]), .rready(s_rready[gi]),
            .digest(m_digest[gi]), .done(m_done[gi])
        );
    end
endgenerate

// ---- crossbar master-side buses ----
wire [M_COUNT*M_ID_WIDTH-1:0] m_axi_awid;
wire [M_COUNT*ADDR_WIDTH-1:0] m_axi_awaddr;
wire [M_COUNT*8-1:0]          m_axi_awlen;
wire [M_COUNT*3-1:0]          m_axi_awsize;
wire [M_COUNT*2-1:0]          m_axi_awburst;
wire [M_COUNT-1:0]            m_axi_awlock;
wire [M_COUNT*4-1:0]          m_axi_awcache;
wire [M_COUNT*3-1:0]          m_axi_awprot;
wire [M_COUNT*4-1:0]          m_axi_awqos;
wire [M_COUNT*4-1:0]          m_axi_awregion;
wire [M_COUNT-1:0]            m_axi_awvalid;
wire [M_COUNT-1:0]            m_axi_awready;
wire [M_COUNT*DATA_WIDTH-1:0] m_axi_wdata;
wire [M_COUNT*STRB_WIDTH-1:0] m_axi_wstrb;
wire [M_COUNT-1:0]            m_axi_wlast;
wire [M_COUNT-1:0]            m_axi_wvalid;
wire [M_COUNT-1:0]            m_axi_wready;
wire [M_COUNT*M_ID_WIDTH-1:0] m_axi_bid;
wire [M_COUNT*2-1:0]          m_axi_bresp;
wire [M_COUNT-1:0]            m_axi_bvalid;
wire [M_COUNT-1:0]            m_axi_bready;
wire [M_COUNT*M_ID_WIDTH-1:0] m_axi_arid;
wire [M_COUNT*ADDR_WIDTH-1:0] m_axi_araddr;
wire [M_COUNT*8-1:0]          m_axi_arlen;
wire [M_COUNT*3-1:0]          m_axi_arsize;
wire [M_COUNT*2-1:0]          m_axi_arburst;
wire [M_COUNT-1:0]            m_axi_arlock;
wire [M_COUNT*4-1:0]          m_axi_arcache;
wire [M_COUNT*3-1:0]          m_axi_arprot;
wire [M_COUNT*4-1:0]          m_axi_arqos;
wire [M_COUNT*4-1:0]          m_axi_arregion;
wire [M_COUNT-1:0]            m_axi_arvalid;
wire [M_COUNT-1:0]            m_axi_arready;
wire [M_COUNT*M_ID_WIDTH-1:0] m_axi_rid;
wire [M_COUNT*DATA_WIDTH-1:0] m_axi_rdata;
wire [M_COUNT*2-1:0]          m_axi_rresp;
wire [M_COUNT-1:0]            m_axi_rlast;
wire [M_COUNT-1:0]            m_axi_rvalid;
wire [M_COUNT-1:0]            m_axi_rready;

axi_crossbar #(
    .S_COUNT(S_COUNT),
    .M_COUNT(M_COUNT),
    .DATA_WIDTH(DATA_WIDTH),
    .ADDR_WIDTH(ADDR_WIDTH),
    .STRB_WIDTH(STRB_WIDTH),
    .S_ID_WIDTH(S_ID_WIDTH),
    .M_ID_WIDTH(M_ID_WIDTH)
) xbar (
    .clk(clk),
    .rst(rst),

    .s_axi_awid   ({s_awid[1],    s_awid[0]}),
    .s_axi_awaddr ({s_awaddr[1],  s_awaddr[0]}),
    .s_axi_awlen  ({s_awlen[1],   s_awlen[0]}),
    .s_axi_awsize ({s_awsize[1],  s_awsize[0]}),
    .s_axi_awburst({s_awburst[1], s_awburst[0]}),
    .s_axi_awlock ({S_COUNT{1'b0}}),
    .s_axi_awcache({S_COUNT{4'b0011}}),
    .s_axi_awprot ({S_COUNT{3'b000}}),
    .s_axi_awqos  ({S_COUNT{4'b0000}}),
    .s_axi_awuser ({S_COUNT{1'b0}}),
    .s_axi_awvalid(s_awvalid),
    .s_axi_awready(s_awready),
    .s_axi_wdata  ({s_wdata[1], s_wdata[0]}),
    .s_axi_wstrb  ({s_wstrb[1], s_wstrb[0]}),
    .s_axi_wlast  (s_wlast),
    .s_axi_wuser  ({S_COUNT{1'b0}}),
    .s_axi_wvalid (s_wvalid),
    .s_axi_wready (s_wready),
    .s_axi_bid    (s_axi_bid),
    .s_axi_bresp  (s_axi_bresp),
    .s_axi_buser  (),
    .s_axi_bvalid (s_bvalid),
    .s_axi_bready (s_bready),
    .s_axi_arid   ({s_arid[1],    s_arid[0]}),
    .s_axi_araddr ({s_araddr[1],  s_araddr[0]}),
    .s_axi_arlen  ({s_arlen[1],   s_arlen[0]}),
    .s_axi_arsize ({s_arsize[1],  s_arsize[0]}),
    .s_axi_arburst({s_arburst[1], s_arburst[0]}),
    .s_axi_arlock ({S_COUNT{1'b0}}),
    .s_axi_arcache({S_COUNT{4'b0011}}),
    .s_axi_arprot ({S_COUNT{3'b000}}),
    .s_axi_arqos  ({S_COUNT{4'b0000}}),
    .s_axi_aruser ({S_COUNT{1'b0}}),
    .s_axi_arvalid(s_arvalid),
    .s_axi_arready(s_arready),
    .s_axi_rid    (s_axi_rid),
    .s_axi_rdata  (s_axi_rdata),
    .s_axi_rresp  (s_axi_rresp),
    .s_axi_rlast  (s_rlast),
    .s_axi_ruser  (),
    .s_axi_rvalid (s_rvalid),
    .s_axi_rready (s_rready),

    .m_axi_awid   (m_axi_awid),
    .m_axi_awaddr (m_axi_awaddr),
    .m_axi_awlen  (m_axi_awlen),
    .m_axi_awsize (m_axi_awsize),
    .m_axi_awburst(m_axi_awburst),
    .m_axi_awlock (m_axi_awlock),
    .m_axi_awcache(m_axi_awcache),
    .m_axi_awprot (m_axi_awprot),
    .m_axi_awqos  (m_axi_awqos),
    .m_axi_awregion(m_axi_awregion),
    .m_axi_awuser (),
    .m_axi_awvalid(m_axi_awvalid),
    .m_axi_awready(m_axi_awready),
    .m_axi_wdata  (m_axi_wdata),
    .m_axi_wstrb  (m_axi_wstrb),
    .m_axi_wlast  (m_axi_wlast),
    .m_axi_wuser  (),
    .m_axi_wvalid (m_axi_wvalid),
    .m_axi_wready (m_axi_wready),
    .m_axi_bid    (m_axi_bid),
    .m_axi_bresp  (m_axi_bresp),
    .m_axi_buser  ({M_COUNT{1'b0}}),
    .m_axi_bvalid (m_axi_bvalid),
    .m_axi_bready (m_axi_bready),
    .m_axi_arid   (m_axi_arid),
    .m_axi_araddr (m_axi_araddr),
    .m_axi_arlen  (m_axi_arlen),
    .m_axi_arsize (m_axi_arsize),
    .m_axi_arburst(m_axi_arburst),
    .m_axi_arlock (m_axi_arlock),
    .m_axi_arcache(m_axi_arcache),
    .m_axi_arprot (m_axi_arprot),
    .m_axi_arqos  (m_axi_arqos),
    .m_axi_arregion(m_axi_arregion),
    .m_axi_aruser (),
    .m_axi_arvalid(m_axi_arvalid),
    .m_axi_arready(m_axi_arready),
    .m_axi_rid    (m_axi_rid),
    .m_axi_rdata  (m_axi_rdata),
    .m_axi_rresp  (m_axi_rresp),
    .m_axi_rlast  (m_axi_rlast),
    .m_axi_ruser  ({M_COUNT{1'b0}}),
    .m_axi_rvalid (m_axi_rvalid),
    .m_axi_rready (m_axi_rready)
);

genvar gj;
generate
    for (gj = 0; gj < M_COUNT; gj = gj + 1) begin : slv
        axi_ram #(
            .DATA_WIDTH(DATA_WIDTH),
            .ADDR_WIDTH(RAM_ADDR_WIDTH),
            .STRB_WIDTH(STRB_WIDTH),
            .ID_WIDTH(M_ID_WIDTH),
            .PIPELINE_OUTPUT(0)
        ) ram (
            .clk(clk),
            .rst(rst),
            .s_axi_awid   (m_axi_awid[gj*M_ID_WIDTH +: M_ID_WIDTH]),
            .s_axi_awaddr (m_axi_awaddr[gj*ADDR_WIDTH +: RAM_ADDR_WIDTH]),
            .s_axi_awlen  (m_axi_awlen[gj*8 +: 8]),
            .s_axi_awsize (m_axi_awsize[gj*3 +: 3]),
            .s_axi_awburst(m_axi_awburst[gj*2 +: 2]),
            .s_axi_awlock (m_axi_awlock[gj]),
            .s_axi_awcache(m_axi_awcache[gj*4 +: 4]),
            .s_axi_awprot (m_axi_awprot[gj*3 +: 3]),
            .s_axi_awvalid(m_axi_awvalid[gj]),
            .s_axi_awready(m_axi_awready[gj]),
            .s_axi_wdata  (m_axi_wdata[gj*DATA_WIDTH +: DATA_WIDTH]),
            .s_axi_wstrb  (m_axi_wstrb[gj*STRB_WIDTH +: STRB_WIDTH]),
            .s_axi_wlast  (m_axi_wlast[gj]),
            .s_axi_wvalid (m_axi_wvalid[gj]),
            .s_axi_wready (m_axi_wready[gj]),
            .s_axi_bid    (m_axi_bid[gj*M_ID_WIDTH +: M_ID_WIDTH]),
            .s_axi_bresp  (m_axi_bresp[gj*2 +: 2]),
            .s_axi_bvalid (m_axi_bvalid[gj]),
            .s_axi_bready (m_axi_bready[gj]),
            .s_axi_arid   (m_axi_arid[gj*M_ID_WIDTH +: M_ID_WIDTH]),
            .s_axi_araddr (m_axi_araddr[gj*ADDR_WIDTH +: RAM_ADDR_WIDTH]),
            .s_axi_arlen  (m_axi_arlen[gj*8 +: 8]),
            .s_axi_arsize (m_axi_arsize[gj*3 +: 3]),
            .s_axi_arburst(m_axi_arburst[gj*2 +: 2]),
            .s_axi_arlock (m_axi_arlock[gj]),
            .s_axi_arcache(m_axi_arcache[gj*4 +: 4]),
            .s_axi_arprot (m_axi_arprot[gj*3 +: 3]),
            .s_axi_arvalid(m_axi_arvalid[gj]),
            .s_axi_arready(m_axi_arready[gj]),
            .s_axi_rid    (m_axi_rid[gj*M_ID_WIDTH +: M_ID_WIDTH]),
            .s_axi_rdata  (m_axi_rdata[gj*DATA_WIDTH +: DATA_WIDTH]),
            .s_axi_rresp  (m_axi_rresp[gj*2 +: 2]),
            .s_axi_rlast  (m_axi_rlast[gj]),
            .s_axi_rvalid (m_axi_rvalid[gj]),
            .s_axi_rready (m_axi_rready[gj])
        );
    end
endgenerate

// ---- cycle-accurate activity digest ----
reg [63:0] cyc_digest;
reg [63:0] cycles;
reg [63:0] xcount;
// data is only meaningful while its VALID is asserted; masking keeps
// uninitialised skid-buffer contents (X) out of the digest.
wire [31:0] w0 = m_axi_wvalid[0] ? m_axi_wdata[31:0]  : 32'd0;
wire [31:0] w1 = m_axi_wvalid[1] ? m_axi_wdata[63:32] : 32'd0;
wire [31:0] r0 = m_axi_rvalid[0] ? m_axi_rdata[31:0]  : 32'd0;
wire [31:0] r1 = m_axi_rvalid[1] ? m_axi_rdata[63:32] : 32'd0;
wire [63:0] sample = ({w1, w0} ^ {r1, r0}) ^
    {44'd0, m_axi_awvalid, m_axi_awready, m_axi_wvalid, m_axi_wready,
            m_axi_bvalid,  m_axi_arvalid, m_axi_arready, m_axi_rvalid,
            m_axi_rlast,   m_axi_rready};

// The crossbar legitimately drives X on m_axi_wvalid before its first
// transaction; substitute a fixed constant so the digest stays a definite
// value, and count the X cycles separately so a disagreement still shows.
wire        sample_x  = (^sample === 1'bx);
wire [63:0] sample_ok = sample_x ? 64'h9E37_79B9_7F4A_7C15 : sample;

always @(posedge clk) begin
    if (rst) begin
        cyc_digest <= 64'd0;
        cycles     <= 64'd0;
        xcount     <= 64'd0;
    end else begin
        cycles     <= cycles + 64'd1;
        cyc_digest <= {cyc_digest[62:0], cyc_digest[63]} ^ sample_ok;
        xcount     <= xcount + (sample_x ? 64'd1 : 64'd0);
    end
end

reg [63:0] final_digest;

task report;
    begin
        final_digest = (m_digest[0] ^ m_digest[1]) + cyc_digest + cycles
                     + (xcount * 64'd1000003);
        $display("OPS=%0d", num_ops);
        $display("D0=%h D1=%h", m_digest[0], m_digest[1]);
        $display("CYCD=%h CYCLES=%0d XC=%0d", cyc_digest, cycles, xcount);
        $display("DIGEST=%h", final_digest);
    end
endtask

initial begin
    num_ops = 32'd1000;
    if ($value$plusargs("N=%d", num_ops)) begin end
    cyc_digest = 64'd0;
    cycles     = 64'd0;
    xcount     = 64'd0;
    final_digest = 64'd0;
    rst = 1'b1;
    repeat (20) @(posedge clk);
    rst <= 1'b0;
end

reg [63:0] max_cycles;
reg        finished;

initial finished = 1'b0;

always @(posedge clk) begin
    max_cycles = ({32'd0, num_ops} * 64'd512) + 64'd20000;
    if (!finished) begin
        if (cycles > max_cycles) begin
            finished = 1'b1;
            $display("WATCHDOG");
            report;
            $finish;
        end else if (&m_done) begin
            finished = 1'b1;
            report;
            $finish;
        end
    end
end

endmodule
