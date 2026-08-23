// Minimal PicoRV32 testbench. Kept VERBATIM: docs/study/01's published numbers were
// measured with it. The corpus uses tbd.v instead — see RUN.md.
// A tiny word-addressed memory preloaded with a short RV32I loop, so the core
// actually fetches, decodes and executes rather than idling in reset.
`timescale 1ns/1ns
module tb;
	parameter CYCLES = 40000;
	reg clk = 0, resetn = 0;
	wire trap;
	wire mem_valid, mem_instr;
	reg  mem_ready = 0;
	wire [31:0] mem_addr, mem_wdata;
	wire [ 3:0] mem_wstrb;
	reg  [31:0] mem_rdata = 0;

	reg [31:0] mem [0:255];
	integer i;

	picorv32 dut (
		.clk(clk), .resetn(resetn), .trap(trap),
		.mem_valid(mem_valid), .mem_instr(mem_instr), .mem_ready(mem_ready),
		.mem_addr(mem_addr), .mem_wdata(mem_wdata), .mem_wstrb(mem_wstrb),
		.mem_rdata(mem_rdata),
		.pcpi_wr(1'b0), .pcpi_rd(32'b0), .pcpi_wait(1'b0), .pcpi_ready(1'b0),
		.irq(32'b0)
	);

	// One-cycle-latency memory: acknowledge every request with the stored word.
	always @(posedge clk) begin
		if (mem_valid && !mem_ready) begin
			mem_ready <= 1;
			mem_rdata <= mem[mem_addr[9:2]];
			if (mem_wstrb != 4'b0)
				mem[mem_addr[9:2]] <= mem_wdata;
		end else begin
			mem_ready <= 0;
		end
	end

	initial begin
		for (i = 0; i < 256; i = i + 1) mem[i] = 32'h00000013; // nop
		mem[0]  = 32'h00100093; // addi x1, x0, 1
		mem[1]  = 32'h00200113; // addi x2, x0, 2
		mem[2]  = 32'h002081b3; // add  x3, x1, x2
		mem[3]  = 32'h00318233; // add  x4, x3, x3
		mem[4]  = 32'h004202b3; // add  x5, x4, x4
		mem[5]  = 32'h00000063; // beq  x0, x0, 0  (loop back)
		resetn = 0;
		repeat (4) @(posedge clk);
		resetn = 1;
	end

	always #5 clk = ~clk;

	initial begin
		repeat (CYCLES) @(posedge clk);
		$display("trap=%b addr=%h", trap, mem_addr);
		$finish;
	end
endmodule
