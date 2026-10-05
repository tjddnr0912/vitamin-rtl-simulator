`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  task automatic rev(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside [4'd3:4'd1]: m = 1; default: m = 0; endcase
    $display("rev v=%0d m=%0d", v, m);
  endtask
  task automatic multi(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 4'd1, 4'd3, [4'd8:4'd9]: m = 1; 4'd2, 4'b11??: m = 2; default: m = 0; endcase
    $display("multi v=%0d m=%0d", v, m);
  endtask
  task automatic overlap(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside [4'd0:4'd7]: m = 1; 4'd5: m = 2; 4'b01??: m = 3; 4'b1???: m = 4; default: m = 0; endcase
    $display("overlap v=%0d m=%0d", v, m);
  endtask
  task automatic nodef(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 4'd1: m = 1; [4'd2:4'd3]: m = 2; endcase
    $display("nodef v=%b m=%0d", v, m);
  endtask
  initial begin
    rev(4'd1); rev(4'd2); rev(4'd3);
    multi(4'd1); multi(4'd3); multi(4'd8); multi(4'd9); multi(4'd2); multi(4'd12); multi(4'd15); multi(4'd4);
    overlap(4'd5); overlap(4'd6); overlap(4'd8); overlap(4'd9);
    nodef(4'd1); nodef(4'd2); nodef(4'd4); nodef(4'bxx00);
    $finish;
  end
endmodule
