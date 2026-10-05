`timescale 1ns/1ns
module top;
  logic [7:0] v8;
  localparam logic [3:0] A = 4'd15;
  int m1, m2, m3, m4, m5, m6, m7, m8;
  initial begin
    v8 = 8'hFC;
    case (v8) inside ~4'b0011: m1 = 1; default: m1 = 0; endcase
    m2 = (v8 inside {~4'b0011});
    v8 = 8'h10;
    case (v8) inside 4'd15 + 4'd1: m3 = 1; default: m3 = 0; endcase
    case (v8) inside A + 4'd1: m4 = 1; default: m4 = 0; endcase
    case (v8) inside [4'd15 + 4'd1 : 8'h10]: m5 = 1; default: m5 = 0; endcase
    case (v8) inside 1'b1 ? 4'd15 + 4'd1 : 4'd0: m7 = 1; default: m7 = 0; endcase
    case (v8) inside 4'd8 << 1: m8 = 1; default: m8 = 0; endcase
    v8 = 8'hFF;
    case (v8) inside -4'd1: m6 = 1; default: m6 = 0; endcase
    $display("m1=%0d m2=%0d m3=%0d m4=%0d m5=%0d m6=%0d m7=%0d m8=%0d", m1, m2, m3, m4, m5, m6, m7, m8);
    $finish;
  end
  initial #1000 $finish;
endmodule
