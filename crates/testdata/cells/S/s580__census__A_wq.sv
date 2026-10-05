`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("A01q %b", v ==? 4'b1100);
    v=4'b1100; $display("A02q %b", v ==? 4'b1?00);
    v=4'b1000; $display("A03q %b", v ==? 4'b1?00);
    v=4'b0100; $display("A04q %b", v ==? 4'b1?00);
    v=4'b1100; $display("A07q %b", v ==? 4'b1x00);
    v=4'b1100; $display("A08q %b", v ==? 4'b1z00);
    v=4'b0100; $display("A09q %b", v ==? 4'b?100);
    v=4'b1100; $display("A10q %b", v ==? 4'bx100);
    v=4'b0100; $display("A11q %b", v ==? 4'bz100);
    v=4'b1101; $display("A12q %b", v ==? 4'b110?);
    v=4'b1010; $display("A13q %b", v ==? 4'b????);
    v=4'b1010; $display("A14q %b", v ==? 4'bxxxx);
    v=4'b1100; $display("A20q %b", v ==? 3'b1?0);
    v=4'b0110; $display("A21q %b", v ==? 3'b1?0);
    v=4'b1100; $display("A22q %b", v ==? 6'b001?00);
    v=4'b1100; $display("A23q %b", v ==? 6'b101?00);
    v=4'b1100; $display("A24q %b", v ==? 6'b??1?00);
    v=4'b1100; $display("A28q %b", v ==? 4'hC);
    v=4'b1100; $display("A29q %b", v ==? 4'h?);
    v=4'b1100; $display("A30q %b", v ==? 4'hx);
    v=4'b1100; $display("A31q %b", v ==? 4'dx);
    v=4'b1100; $display("A32q %b", v ==? 4'o1?);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
