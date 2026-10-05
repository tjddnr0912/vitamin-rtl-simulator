`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  localparam logic [3:0] P = 4'b1000;
  logic [71:0] w72; logic [3:0] v; logic [35:0] w36; int m;
  initial begin
    m = 9; case (4'b1100) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("const m=%0d", m);
    m = 9; case (P) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("param m=%0d", m);
    w72 = 72'h80_0000_0000_0000_0050; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h80_0000_0000_0000_0051; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h2; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    w72 = 72'h0; m = 9;
    case (w72) inside 72'h80_0000_0000_0000_00?0: m = 1; [72'h1:72'h3]: m = 2; default: m = 0; endcase
    $display("w72=%h m=%0d", w72, m);
    v = 4'bxxxx; m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    v = 4'bzzzz; m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    v = 4'd3;    m = 9; case (v) inside 4'b????: m = 1; default: m = 0; endcase $display("allq v=%b m=%0d", v, m);
    w36 = 36'h1; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    w36 = 36'h3; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    w36 = 36'h2; m = 9; case (w36) inside 'bx1: m = 1; default: m = 0; endcase $display("bx1 w36=%h m=%0d", w36, m);
    $finish;
  end
endmodule
