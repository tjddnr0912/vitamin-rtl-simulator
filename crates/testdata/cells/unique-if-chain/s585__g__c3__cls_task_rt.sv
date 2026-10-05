class C;
  int r;
  task t(input int x);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endtask
endclass
module top;
  C c;
  initial begin c = new; #1 c.t(0); $display("t=%0t r=%0d", $time, c.r); #1 $finish; end
endmodule
