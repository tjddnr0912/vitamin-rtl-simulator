// 001_alu.sv — 8-bit combinational ALU.
//
// A purely combinational DUT (`always @*`) selecting one of four operations
// with a 2-bit op code, plus a testbench that exercises each op and $display's
// the result.
//
// Run:
//   vita examples/001_alu.sv
// Produces: alu.vcd

`timescale 1ns/1ns

// op encoding: 0=ADD, 1=SUB, 2=AND, 3=OR.
module alu (
    input      [7:0] a,
    input      [7:0] b,
    input      [1:0] op,
    output reg [7:0] y
);
    always @* begin
        case (op)
            2'd0: y = a + b;
            2'd1: y = a - b;
            2'd2: y = a & b;
            2'd3: y = a | b;
        endcase
    end
endmodule

module tb;
    reg  [7:0] a, b;
    reg  [1:0] op;
    wire [7:0] y;

    alu dut (.a(a), .b(b), .op(op), .y(y));

    initial begin
        $dumpfile("alu.vcd");
        $dumpvars(0, tb);

        a = 8'hF0;
        b = 8'h0F;

        op = 2'd0; #1 $display("ADD: %h + %h = %h (%0d)", a, b, y, y);
        op = 2'd1; #1 $display("SUB: %h - %h = %h (%0d)", a, b, y, y);
        op = 2'd2; #1 $display("AND: %h & %h = %h", a, b, y);
        op = 2'd3; #1 $display("OR : %h | %h = %h", a, b, y);

        // A second vector with a borrow in the subtract.
        a = 8'd10;
        b = 8'd25;
        op = 2'd1; #1 $display("SUB: %0d - %0d = %0d (wraps mod 256)", a, b, y);

        $finish;
    end
endmodule
