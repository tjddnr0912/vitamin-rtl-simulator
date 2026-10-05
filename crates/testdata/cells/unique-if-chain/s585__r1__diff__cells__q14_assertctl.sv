module top;
  logic a = 0, b = 0; logic [1:0] y = 0;
  initial begin
    #1 $assertcontrol(4, 32);
    unique if (a) y = 1; else if (b) y = 2;
    unique if (a) y = 1;
    priority if (a) y = 1; else if (b) y = 2;
    #1 $assertcontrol(3, 32);
    unique if (a) y = 1; else if (b) y = 2;
    unique if (a) y = 1;
    #1 $assertcontrol(4, 128);
    priority if (a) y = 1; else if (b) y = 2;
    priority if (a) y = 1;
    unique if (a) y = 1; else if (b) y = 2;
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
endmodule
