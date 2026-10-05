`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [7:0] v8; bit [3:0] a4, b4; int m;
  initial begin
    a4 = 4'h8; b4 = 4'h8; v8 = 8'h10;
    case (v8) inside a4 + b4: m = 1; default: m = 0; endcase $display("sum m=%0d", m);
    case (v8) inside [a4 + b4 : 8'hFF]: m = 1; default: m = 0; endcase $display("sumrange m=%0d", m);
    v8 = 8'hF7;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("notF7 m=%0d", m);
    v8 = 8'h07;
    case (v8) inside ~a4: m = 1; default: m = 0; endcase $display("not07 m=%0d", m);
    $finish;
  end
endmodule
