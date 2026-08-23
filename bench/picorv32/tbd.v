// PicoRV32 workload-corpus testbench.
//
// Distinct from tb.v, which is kept verbatim because study/01's published numbers
// were measured with it. tb.v prints `trap=%b addr=%h` — FINAL STATE ONLY, which is
// blind to any divergence the core later overwrites. This one folds the whole memory
// bus into an accumulator on every cycle, so the digest is a cycle-resolution
// differential gate rather than an end-of-run snapshot.
`timescale 1ns/1ns
module tb;
	reg clk = 0, resetn = 0;
	wire trap;
	wire mem_valid, mem_instr;
	reg  mem_ready = 0;
	wire [31:0] mem_addr, mem_wdata;
	wire [ 3:0] mem_wstrb;
	reg  [31:0] mem_rdata = 0;
	reg  [63:0] acc = 64'd0;
	integer i, cycles, got;

	reg [31:0] mem [0:255];

	picorv32 dut (
		.clk(clk), .resetn(resetn), .trap(trap),
		.mem_valid(mem_valid), .mem_instr(mem_instr), .mem_ready(mem_ready),
		.mem_addr(mem_addr), .mem_wdata(mem_wdata), .mem_wstrb(mem_wstrb),
		.mem_rdata(mem_rdata),
		.pcpi_wr(1'b0), .pcpi_rd(32'b0), .pcpi_wait(1'b0), .pcpi_ready(1'b0),
		.irq(32'b0)
	);

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

	// Rotate-xor so that WHEN a value appears matters, not just which values did.
	//
	// The bus payload is folded in ONLY while `mem_valid` holds it, and `mem_wdata`
	// only while a strobe selects it: picorv32 drives both to x otherwise, and an
	// ungated accumulator xors x into every bit within one cycle — the digest comes
	// back `xxxxxxxxxxxxxxxx` and compares equal to nothing. The handshake bits are
	// folded unconditionally, which is what keeps the digest cycle-resolution.
	always @(posedge clk) if (resetn)
		acc <= {acc[62:0], acc[63]}
		     ^ (mem_valid ? {mem_addr, (|mem_wstrb ? mem_wdata : 32'd0)} : 64'd0)
		     ^ {59'd0, (trap === 1'b1), mem_valid,
		        (mem_valid && mem_instr), (mem_ready === 1'b1),
		        (mem_valid && |mem_wstrb)};

	initial begin
		got = $value$plusargs("N=%d", cycles);
		if (got == 0) cycles = 1500000;
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
		@(posedge resetn);
		repeat (cycles) @(posedge clk);
		$display("DIGEST=%h", acc);
		$finish;
	end
endmodule
