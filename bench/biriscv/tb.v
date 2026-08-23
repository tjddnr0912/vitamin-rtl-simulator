//-----------------------------------------------------------------
// biriscv workload bench testbench (derived from upstream
// tb/tb_core_icarus/tb_top.v, Apache-2.0, ultraembedded/biriscv)
// Deterministic, self-terminating, emits one DIGEST line.
//-----------------------------------------------------------------
module tb_top;

reg clk;
reg rst;

reg [7:0] mem[0:11855];
integer i;
integer ncycles;
integer cyc;

reg [63:0] dig_one;   // xor-accumulated "bit is 1" mask
reg [63:0] dig_known; // xor-accumulated "bit is not x/z" mask

initial
begin
    ncycles = 50000;
    if ($value$plusargs("N=%d", ncycles)) ;

    clk = 0;
    rst = 1;
    dig_one   = 64'd0;
    dig_known = 64'd0;
    cyc = 0;

    for (i=0;i<11856;i=i+1)
        mem[i] = 8'h00;
    $readmemh("prog.hex", mem);
    for (i=0;i<11856;i=i+1)
        u_mem.write(i, mem[i]);

    repeat (5) @(posedge clk);
    #1 rst = 0;
end

initial
begin
    forever
    begin
        clk = #5 ~clk;
    end
end

wire          mem_i_rd_w;
wire          mem_i_flush_w;
wire          mem_i_invalidate_w;
wire [ 31:0]  mem_i_pc_w;
wire [ 31:0]  mem_d_addr_w;
wire [ 31:0]  mem_d_data_wr_w;
wire          mem_d_rd_w;
wire [  3:0]  mem_d_wr_w;
wire          mem_d_cacheable_w;
wire [ 10:0]  mem_d_req_tag_w;
wire          mem_d_invalidate_w;
wire          mem_d_writeback_w;
wire          mem_d_flush_w;
wire          mem_i_accept_w;
wire          mem_i_valid_w;
wire          mem_i_error_w;
wire [ 63:0]  mem_i_inst_w;
wire [ 31:0]  mem_d_data_rd_w;
wire          mem_d_accept_w;
wire          mem_d_ack_w;
wire          mem_d_error_w;
wire [ 10:0]  mem_d_resp_tag_w;

// per-cycle observation vector: everything the core drives
wire [63:0] obs_w = {mem_i_pc_w, mem_d_addr_w}
                  ^ {mem_d_data_wr_w, mem_d_data_rd_w}
                  ^ {21'd0, mem_d_req_tag_w, mem_d_rd_w, mem_d_wr_w,
                     mem_d_cacheable_w, mem_d_invalidate_w, mem_d_writeback_w,
                     mem_d_flush_w, mem_i_rd_w, mem_i_flush_w,
                     mem_i_invalidate_w, mem_i_valid_w, 21'd0};

function [63:0] ones_mask;
    input [63:0] v;
    integer k;
    begin
        for (k=0;k<64;k=k+1)
            ones_mask[k] = (v[k] === 1'b1);
    end
endfunction

function [63:0] known_mask;
    input [63:0] v;
    integer k;
    begin
        for (k=0;k<64;k=k+1)
            known_mask[k] = (v[k] === 1'b1) || (v[k] === 1'b0);
    end
endfunction

always @(posedge clk)
if (!rst)
begin
    dig_one   <= {dig_one[62:0],   dig_one[63]}   ^ ones_mask(obs_w);
    dig_known <= {dig_known[58:0], dig_known[63:59]} ^ known_mask(obs_w);
    cyc       <= cyc + 1;
    if (cyc >= ncycles)
    begin
        $display("CYCLES=%0d", cyc);
        $display("DIGEST=%08x%08x", dig_one[63:32] ^ dig_known[31:0],
                                    dig_one[31:0]  ^ dig_known[63:32]);
        $finish;
    end
end

initial
begin
    #200000000;
    $display("WATCHDOG");
    $display("DIGEST=deadbeefdeadbeef");
    $finish;
end

riscv_core
u_dut
(
     .clk_i(clk)
    ,.rst_i(rst)
    ,.mem_d_data_rd_i(mem_d_data_rd_w)
    ,.mem_d_accept_i(mem_d_accept_w)
    ,.mem_d_ack_i(mem_d_ack_w)
    ,.mem_d_error_i(mem_d_error_w)
    ,.mem_d_resp_tag_i(mem_d_resp_tag_w)
    ,.mem_i_accept_i(mem_i_accept_w)
    ,.mem_i_valid_i(mem_i_valid_w)
    ,.mem_i_error_i(mem_i_error_w)
    ,.mem_i_inst_i(mem_i_inst_w)
    ,.intr_i(1'b0)
    ,.reset_vector_i(32'h80000000)
    ,.cpu_id_i(32'b0)
    ,.mem_d_addr_o(mem_d_addr_w)
    ,.mem_d_data_wr_o(mem_d_data_wr_w)
    ,.mem_d_rd_o(mem_d_rd_w)
    ,.mem_d_wr_o(mem_d_wr_w)
    ,.mem_d_cacheable_o(mem_d_cacheable_w)
    ,.mem_d_req_tag_o(mem_d_req_tag_w)
    ,.mem_d_invalidate_o(mem_d_invalidate_w)
    ,.mem_d_writeback_o(mem_d_writeback_w)
    ,.mem_d_flush_o(mem_d_flush_w)
    ,.mem_i_rd_o(mem_i_rd_w)
    ,.mem_i_flush_o(mem_i_flush_w)
    ,.mem_i_invalidate_o(mem_i_invalidate_w)
    ,.mem_i_pc_o(mem_i_pc_w)
);

tcm_mem
u_mem
(
     .clk_i(clk)
    ,.rst_i(rst)
    ,.mem_i_rd_i(mem_i_rd_w)
    ,.mem_i_flush_i(mem_i_flush_w)
    ,.mem_i_invalidate_i(mem_i_invalidate_w)
    ,.mem_i_pc_i(mem_i_pc_w)
    ,.mem_d_addr_i(mem_d_addr_w)
    ,.mem_d_data_wr_i(mem_d_data_wr_w)
    ,.mem_d_rd_i(mem_d_rd_w)
    ,.mem_d_wr_i(mem_d_wr_w)
    ,.mem_d_cacheable_i(mem_d_cacheable_w)
    ,.mem_d_req_tag_i(mem_d_req_tag_w)
    ,.mem_d_invalidate_i(mem_d_invalidate_w)
    ,.mem_d_writeback_i(mem_d_writeback_w)
    ,.mem_d_flush_i(mem_d_flush_w)
    ,.mem_i_accept_o(mem_i_accept_w)
    ,.mem_i_valid_o(mem_i_valid_w)
    ,.mem_i_error_o(mem_i_error_w)
    ,.mem_i_inst_o(mem_i_inst_w)
    ,.mem_d_data_rd_o(mem_d_data_rd_w)
    ,.mem_d_accept_o(mem_d_accept_w)
    ,.mem_d_ack_o(mem_d_ack_w)
    ,.mem_d_error_o(mem_d_error_w)
    ,.mem_d_resp_tag_o(mem_d_resp_tag_w)
);

endmodule
