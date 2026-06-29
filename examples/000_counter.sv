// 000_counter.sv — 4-bit synchronous up-counter with active-high reset.
//
// The canonical vitamin quickstart. A clocked counter DUT plus a testbench
// that drives clk/rst, $display's the count each cycle, and dumps a VCD.
//
// Run:
//   vita examples/000_counter.sv
// Produces: counter.vcd (open in GTKWave / Surfer)

`timescale 1ns/1ns

// DUT: increments on every rising clock edge; active-high synchronous reset
// forces the count back to 0.
module counter #(parameter WIDTH = 4) (
    input              clk,
    input              rst,
    output reg [WIDTH-1:0] cnt
);
    always @(posedge clk) begin
        if (rst)
            cnt <= {WIDTH{1'b0}};
        else
            cnt <= cnt + 1'b1;
    end
endmodule

// Testbench: free-running clock, pulse reset, then watch the counter climb.
module tb;
    reg        clk;
    reg        rst;
    wire [3:0] cnt;
    integer    i;

    counter #(.WIDTH(4)) dut (.clk(clk), .rst(rst), .cnt(cnt));

    // 10ns clock period.
    initial clk = 1'b0;
    always #5 clk = ~clk;

    initial begin
        $dumpfile("counter.vcd");
        $dumpvars(0, tb);

        // Hold reset across the first rising edge, then release.
        rst = 1'b1;
        @(posedge clk);          // cnt loads 0 here
        @(negedge clk);
        rst = 1'b0;

        // Count up for 12 cycles, printing the value after each edge.
        for (i = 0; i < 12; i = i + 1) begin
            @(posedge clk);
            #1 $display("t=%0t  cnt=%0d (0x%h)", $time, cnt, cnt);
        end

        $display("done: final cnt=%0d", cnt);
        $finish;
    end
endmodule
