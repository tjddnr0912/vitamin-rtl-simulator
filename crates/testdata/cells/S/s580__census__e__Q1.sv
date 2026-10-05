`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [35:0] w; logic [1:0] p; localparam logic [3:0] P = 4'b1100;
  initial begin
    v=4'b1100; $display("E01q %b", v ==? 'b1?00);
    v=4'b0011; $display("E02q %b", v ==? 'bx1);
    v=4'b0010; $display("E03q %b", v ==? 'bx1);
    v=4'b1100; $display("E04q %b", v ==? 'b?);
    w=36'hF_0000_0001; $display("E05q %b", w ==? 'bx1);
    w=36'hF_0000_0001; $display("E06q %b", w ==? 'b1x1);
    w=36'h0_0000_0001; $display("E05r %b", w ==? 'bx1);
    w=36'hF_0000_0001; $display("E05s %b", w ==? 36'bx1);
    v=4'b1100; $display("E07q %b", v ==? 'h?);
    v=4'b1010; $display("E10q %b", v ==? 'x);
    v=4'b1010; $display("E11q %b", v ==? 'z);
    v=4'b0000; $display("E15q %b", (v ==? 4'b0001) || (v ==? 'x));
    #1 $finish;
  end
endmodule
