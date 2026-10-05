`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1x00; $display("X01q %b", v ==? 4'b1?00);
    v=4'bx100; $display("X02q %b", v ==? 4'b1?00);
    v=4'bz100; $display("X03q %b", v ==? 4'b1?00);
    v=4'b1z00; $display("X04q %b", v ==? 4'b1?00);
    v=4'bx100; $display("X07q %b", v ==? 4'b?100);
    v=4'bxxxx; $display("X08q %b", v ==? 4'b????);
    v=4'bx100; $display("X09q %b", v ==? 4'b0100);
    v=4'b1x00; $display("X11q %b", v ==? 4'b1x00);
    v=4'bx100; $display("X12q %b", v ==? 4'b0?00);
    v=4'bx110; $display("X13q %b", v ==? 4'b0?00);
    v=4'bzzzz; $display("X16q %b", v ==? 4'bzzzz);
    v=4'bzzzz; $display("X17q %b", v ==? 4'b0000);
    v=4'bx100; $display("X18q %b", v ==? 6'b??1?00);
    v=4'bx100; $display("X19q %b", v ==? 6'b011?00);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
