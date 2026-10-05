`timescale 1ns/1ns
module top;
  logic [32:0] e33; logic [64:0] e65; logic [127:0] e128; logic [30:0] e31; logic [63:0] e64;
  logic signed [127:0] s128; logic signed [64:0] s65; logic signed [32:0] s33;
  int m2, m3, m4, m5, m6, m7, m8, n1, n2, n3, n4;
  initial begin
    e33 = 33'h1_0000_0000;
    case (e33) inside 'd1 << 32: m2 = 1; default: m2 = 0; endcase
    case (e33) inside [32'hFFFF_FFFF + 32'd1 : 33'h1_0000_0000]: m3 = 1; default: m3 = 0; endcase
    e65 = {1'b1, 64'h0};
    case (e65) inside 64'd1 << 64: m4 = 1; default: m4 = 0; endcase
    case (e65) inside 64'hFFFF_FFFF_FFFF_FFFF + 64'd1: m5 = 1; default: m5 = 0; endcase
    e128 = 128'h8000_0000_0000_0000_0000_0000_0000_0000;
    case (e128) inside 128'd1 << 127: m6 = 1; [0:5]: m6 = 2; default: m6 = 0; endcase
    e31 = 31'h7FFF_FFFF;
    case (e31) inside -1: m7 = 1; 31'h7FFF_FFFF: m7 = 2; default: m7 = 0; endcase
    e64 = 64'hFFFF_FFFF_FFFF_FFFF;
    case (e64) inside -64'sd1: m8 = 1; default: m8 = 0; endcase
    s128 = -128'sd5; s65 = -65'sd1; s33 = -33'sd1;
    case (s128) inside [-10:-1]: n1 = 1; default: n1 = 0; endcase
    case (s65) inside [-2:0]: n2 = 1; default: n2 = 0; endcase
    case (s33) inside -1: n3 = 1; default: n3 = 0; endcase
    case (s128) inside [-128'sd4 : 128'sd4], -128'sd6: n4 = 1; default: n4 = 0; endcase
    $display("m2=%0d m3=%0d m4=%0d m5=%0d m6=%0d m7=%0d m8=%0d n1=%0d n2=%0d n3=%0d n4=%0d", m2, m3, m4, m5, m6, m7, m8, n1, n2, n3, n4);
    $finish;
  end
  initial #1000 $finish;
endmodule
