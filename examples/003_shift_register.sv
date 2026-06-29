// 003_shift_register.sv — 8-bit shift register with serial in / serial out.
//
// On each rising clock edge the register shifts left by one, taking `sin` into
// the LSB; the MSB falls out on `sout`. The testbench streams a byte in bit by
// bit (MSB first), then reads it back out, $display'ing the captured value.
//
// Run:
//   vita examples/003_shift_register.sv
// Produces: shift_register.vcd

`timescale 1ns/1ns

module shift_reg #(parameter WIDTH = 8) (
    input                  clk,
    input                  rst,
    input                  sin,    // serial in
    output                 sout,   // serial out (MSB)
    output reg [WIDTH-1:0] q       // parallel view of the register
);
    assign sout = q[WIDTH-1];

    always @(posedge clk) begin
        if (rst)
            q <= {WIDTH{1'b0}};
        else
            q <= {q[WIDTH-2:0], sin};   // shift left, sin into LSB
    end
endmodule

module tb;
    reg        clk;
    reg        rst;
    reg        sin;
    wire       sout;
    wire [7:0] q;
    integer    i;

    // The byte we will stream in, MSB first.
    reg [7:0]  pattern = 8'b1011_0010;
    reg [7:0]  captured;

    shift_reg #(.WIDTH(8)) dut (
        .clk(clk), .rst(rst), .sin(sin), .sout(sout), .q(q)
    );

    initial clk = 1'b0;
    always #5 clk = ~clk;

    initial begin
        $dumpfile("shift_register.vcd");
        $dumpvars(0, tb);

        rst = 1'b1;
        sin = 1'b0;
        @(posedge clk);          // clear the register
        @(negedge clk);
        rst = 1'b0;

        // Shift the pattern in, MSB (bit 7) first.
        for (i = 7; i >= 0; i = i - 1) begin
            sin = pattern[i];
            @(posedge clk);
            #1 $display("in=%b -> q=%b sout=%b", sin, q, sout);
        end

        captured = q;
        $display("captured byte = %b (0x%h)", captured, captured);
        if (captured === pattern)
            $display("PASS: register holds the streamed pattern");
        else
            $display("FAIL: expected %b", pattern);

        $finish;
    end
endmodule
