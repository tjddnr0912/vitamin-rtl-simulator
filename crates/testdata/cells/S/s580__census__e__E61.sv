`timescale 1ns/1ns
module t;
  string s;
  initial begin
    s="ab"; $display("E61 %b", s inside {"ab", "cd"});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
