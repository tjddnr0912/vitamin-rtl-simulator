module top;
  bit [3:0] x; bit [7:0] y1, y2;
  function automatic bit [7:0] cf(input bit [3:0] v);
    case (v) inside [4'd0:4'd3]: return 8'd10; 4'b1?00: return 8'd20; default: return 8'd30; endcase
  endfunction
  function bit [7:0] cs(input bit [3:0] v);  // static function
    case (v + 4'd1) inside [4'd0:4'd3]: cs = 8'd11; 4'b1?01: cs = 8'd21; default: cs = 8'd31; endcase
  endfunction
  assign y1 = cf(x);
  assign y2 = cs(x);
  class C;
    bit [3:0] k; bit [3:0] lo, hi;
    function new(); k = 4'd5; lo = 4'd4; hi = 4'd6; endfunction
    function int meth();
      case (this.k) inside [lo:hi]: return 1; 4'b11??: return 2; default: return 0; endcase
    endfunction
    static function int sm(input bit [3:0] v);
      case (v) inside [4'd8:4'd15]: return 3; default: return 4; endcase
    endfunction
  endclass
  C c;
  initial begin
    c = new();
    x = 4'd2; #1 $display("t1 y1=%0d y2=%0d", y1, y2);
    x = 4'd12; #1 $display("t2 y1=%0d y2=%0d", y1, y2);
    x = 4'd8; #1 $display("t3 y1=%0d y2=%0d", y1, y2);
    $display("meth=%0d", c.meth());
    c.k = 4'd13; $display("meth2=%0d", c.meth());
    $display("sm=%0d %0d", C::sm(4'd9), C::sm(4'd1));
    #1 $finish;
  end
endmodule
