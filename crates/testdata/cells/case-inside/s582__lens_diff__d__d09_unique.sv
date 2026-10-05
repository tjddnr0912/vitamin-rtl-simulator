`timescale 1ns/1ns
module top;
  logic [3:0] x;
  int m;
  initial begin
    x = 4'd7; m = 0;
    unique case (x) inside [0:3]: m = 1; 4'd9: m = 2; endcase
    $display("U1ci m=%0d", m);
    unique case (x) 4'd0, 4'd1, 4'd2, 4'd3: m = 1; 4'd9: m = 2; endcase
    $display("U1pl m=%0d", m);
    priority case (x) inside [0:3]: m = 1; endcase
    $display("P1ci m=%0d", m);
    priority case (x) 4'd0, 4'd1, 4'd2, 4'd3: m = 1; endcase
    $display("P1pl m=%0d", m);
    unique0 case (x) inside [0:3]: m = 1; endcase
    $display("U0ci m=%0d", m);
    unique0 case (x) 4'd0, 4'd1: m = 1; endcase
    $display("U0pl m=%0d", m);
    unique case (x) inside [0:3]: m = 1; default: m = 9; endcase
    $display("UDci m=%0d", m);
    m = 0;
    unique case (x) inside [4:8]: m = 1; [6:7]: m = 2; endcase
    $display("UOci m=%0d", m);
    m = 0;
    unique case (x) 4'd4, 4'd5, 4'd6, 4'd7: m = 1; 4'd6, 4'd7: m = 2; endcase
    $display("UOpl m=%0d", m);
    $finish;
  end
  initial #1000 $finish;
endmodule
