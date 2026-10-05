`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; logic [7:0] w; int m;
  task automatic a(input logic [3:0] x);
    v = x; m = 9;
    case (v) inside 8'b0000_1?00: m = 1; 8'h1F: m = 2; [8'd5:8'd6]: m = 3; 8'b1???_??11: m = 4; default: m = 0; endcase
    $display("a v=%b m=%0d", v, m);
  endtask
  task automatic b(input logic [7:0] x);
    w = x; m = 9;
    case (w) inside 4'b1?00: m = 1; [4'd1:4'd2]: m = 2; default: m = 0; endcase
    $display("b w=%h m=%0d", w, m);
  endtask
  initial begin
    a(4'b1100); a(4'b1000); a(4'b1111); a(4'd5); a(4'd6); a(4'b0011);
    b(8'h0C); b(8'h1C); b(8'h08); b(8'h01); b(8'h11); b(8'hF8);
    $finish;
  end
endmodule
