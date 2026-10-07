module top;
  class c;
    rand logic [7:0] m;
    logic two;
    constraint k {
      if (two) { $countones(m) == 2; } else { $countones(m) == 1; }
    }
  endclass
  initial begin
    c o = new;
    o.two = 1'b1;
    void'(o.randomize());
    $display("ones=%0d", $countones(o.m));
    $finish;
  end
endmodule
